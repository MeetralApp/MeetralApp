//! Drop-engine-lock-before-join helpers for audio capture teardown.
//!
//! Pattern: short lock → take `CaptureHandle` → drop guard →
//! [`crate::runtime::direct_relay::join_capture_handle`] → re-lock to start/publish.

use crate::audio::{AudioDeviceInfo, CaptureHandle};
use crate::config::AppConfig;
use crate::pipeline::inbound::{teardown_inbound_parts, InboundTeardown};
use crate::pipeline::outbound::{teardown_outbound_parts, OutboundTeardown};
use crate::runtime::direct_relay::join_capture_handle;
use tauri::AppHandle;

use super::{
    audio_backoff_ms, unix_ms_now_u64, Direction, PipelineState, SharedEngine, TranslationEngine,
    AUDIO_DIRECT_LOST_MESSAGE, AUDIO_INBOUND_LOST_MESSAGE, AUDIO_OUTBOUND_LOST_MESSAGE,
    AUDIO_RECOVERY_CONFIRM_MS, MAX_AUDIO_RECONNECT_ATTEMPTS,
};

pub(super) async fn join_optional_capture(handle: Option<CaptureHandle>) {
    if let Some(handle) = handle {
        join_capture_handle(handle).await;
    }
}

/// Async pipeline teardown taken under a short engine lock; `run` executes
/// after the guard is dropped so bridge aborts / TTS joins never stall IPC.
pub(super) enum SideTeardown {
    None,
    Outbound(OutboundTeardown),
    Inbound(InboundTeardown),
}

impl SideTeardown {
    pub(super) async fn run(self) {
        match self {
            SideTeardown::None => {}
            SideTeardown::Outbound(parts) => teardown_outbound_parts(parts).await,
            SideTeardown::Inbound(parts) => teardown_inbound_parts(parts).await,
        }
    }
}

pub(super) async fn restart_direct_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
) -> bool {
    let handle = {
        let mut guard = engine.lock().await;
        match dir {
            Direction::Outbound => {
                let handle = guard.take_direct_outbound_for_stop();
                guard.status.0 = PipelineState::Off;
                handle
            }
            Direction::Inbound => {
                let handle = guard.take_direct_inbound_for_stop();
                guard.status.1 = PipelineState::Off;
                handle
            }
        }
    };
    join_optional_capture(handle).await;
    let mut guard = engine.lock().await;
    let result = match dir {
        Direction::Outbound if guard.is_outbound_busy() => return false,
        Direction::Inbound if guard.is_inbound_busy() => return false,
        Direction::Outbound => guard.start_direct_outbound(config, devices, app),
        Direction::Inbound => guard.start_direct_inbound(config, devices, app),
    };
    match result {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(?dir, "direct audio restart failed: {e}");
            false
        }
    }
}

pub(super) async fn restart_translate_capture_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> bool {
    let (ok, previous) = {
        let mut guard = engine.lock().await;
        let fault_tx = guard.audio_fault_tx(dir);
        let result = match dir {
            Direction::Outbound => guard.outbound.restart_audio_path(config, devices, fault_tx),
            Direction::Inbound => guard.inbound.restart_audio_path(config, devices, fault_tx),
        };
        match result {
            Ok(previous) => (true, previous),
            Err(e) => {
                tracing::warn!(?dir, "translate capture restart failed: {e:#}");
                (false, None)
            }
        }
    };
    join_optional_capture(previous).await;
    ok
}

enum RecoveryKind {
    Direct,
    Translate,
    None,
}

pub(super) async fn begin_audio_recovery_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
) {
    let (kind, attempt, now_ms) = {
        let mut guard = engine.lock().await;
        if guard.side(dir).audio_path.is_lost() {
            return;
        }
        let now_ms = unix_ms_now_u64();
        if !guard.side(dir).audio_path.next_action_due(now_ms) {
            return;
        }
        let attempt = guard
            .side(dir)
            .audio_path
            .reconnect_attempt()
            .unwrap_or(0)
            .saturating_add(1);
        if attempt > MAX_AUDIO_RECONNECT_ATTEMPTS {
            drop(guard);
            fatal_audio_path_shared(dir, engine, config, devices, app).await;
            return;
        }
        guard.side_mut(dir).audio_path.begin_reconnect(attempt);
        guard.publish_state(app);
        let kind = match dir {
            Direction::Outbound => guard.status.0.clone(),
            Direction::Inbound => guard.status.1.clone(),
        };
        let kind = match kind {
            PipelineState::Direct => RecoveryKind::Direct,
            PipelineState::Active => RecoveryKind::Translate,
            _ => RecoveryKind::None,
        };
        (kind, attempt, now_ms)
    };

    // Resolve-before-kill: only tear down the running audio path when its
    // target devices are resolvable against the current catalog. Tearing down
    // on a mid-churn / transient enumeration is what turned brief endpoint
    // gaps (BT profile switch, virtual-driver engine restart) into
    // user-visible disconnects.
    let target_ready = match kind {
        RecoveryKind::Direct => match dir {
            Direction::Outbound => config.validate_for_direct_outbound(devices).is_ok(),
            Direction::Inbound => config.validate_for_direct_inbound(devices).is_ok(),
        },
        RecoveryKind::Translate => match dir {
            Direction::Outbound => config.validate_for_start_outbound(devices).is_ok(),
            Direction::Inbound => config.validate_for_start_inbound(devices).is_ok(),
        },
        RecoveryKind::None => true,
    };
    if !target_ready {
        tracing::info!(
            ?dir,
            "audio recovery deferred: target devices not resolvable yet"
        );
        let mut guard = engine.lock().await;
        guard
            .side_mut(dir)
            .audio_path
            .set_next_action_ms(now_ms + audio_backoff_ms(attempt));
        guard.publish_state(app);
        return;
    }

    let success = match kind {
        RecoveryKind::Direct => restart_direct_shared(dir, engine, config, devices, app).await,
        RecoveryKind::Translate => {
            restart_translate_capture_shared(dir, engine, config, devices).await
        }
        RecoveryKind::None => false,
    };

    let need_fatal = {
        let mut guard = engine.lock().await;
        if success {
            guard
                .side_mut(dir)
                .audio_path
                .set_next_action_ms(now_ms + AUDIO_RECOVERY_CONFIRM_MS);
            guard.publish_state(app);
            false
        } else {
            guard
                .side_mut(dir)
                .audio_path
                .set_next_action_ms(now_ms + audio_backoff_ms(attempt));
            let fatal = attempt >= MAX_AUDIO_RECONNECT_ATTEMPTS;
            if !fatal {
                guard.publish_state(app);
            }
            fatal
        }
    };
    if need_fatal {
        fatal_audio_path_shared(dir, engine, config, devices, app).await;
    }
}

pub(super) async fn fatal_audio_path_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
) {
    // Step 1 (short lock): mark lost, mutate state, take teardown parts.
    let (teardown, capture, was_translate) = {
        let mut guard = engine.lock().await;
        guard.side_mut(dir).audio_path.mark_lost(unix_ms_now_u64());
        match dir {
            Direction::Outbound
                if guard.status.0 == PipelineState::Active
                    || guard.status.0 == PipelineState::Starting =>
            {
                guard.reset_mic_mute();
                guard.flush_live_transcript_segments(app, &["outbound"]);
                guard.cancel_all_outbound_tasks();
                guard.side_mut(dir).starting = false;
                let (parts, capture) = guard.outbound.take_teardown_parts();
                guard.side_mut(dir).active_since = None;
                guard.clear_bridge_state(Direction::Outbound);
                guard.last_error = Some(AUDIO_OUTBOUND_LOST_MESSAGE.to_string());
                guard.clear_transcript_relay_if_idle();
                (SideTeardown::Outbound(parts), capture, true)
            }
            Direction::Inbound if guard.status.1 == PipelineState::Active => {
                guard.reset_speaker_mute();
                guard.flush_live_transcript_segments(app, &["inbound"]);
                if let Some(cancel) = guard.side_mut(dir).cancel.take() {
                    cancel.cancel();
                }
                let (parts, capture) = guard.inbound.take_teardown_parts();
                guard.side_mut(dir).active_since = None;
                guard.clear_bridge_state(Direction::Inbound);
                guard.last_error = Some(AUDIO_INBOUND_LOST_MESSAGE.to_string());
                guard.clear_transcript_relay_if_idle();
                (SideTeardown::Inbound(parts), capture, true)
            }
            Direction::Outbound if guard.status.0 == PipelineState::Direct => {
                let capture = guard.take_direct_outbound_for_stop();
                guard.status.0 = PipelineState::Off;
                guard.last_error = Some(AUDIO_DIRECT_LOST_MESSAGE.to_string());
                (SideTeardown::None, capture, false)
            }
            Direction::Inbound if guard.status.1 == PipelineState::Direct => {
                let capture = guard.take_direct_inbound_for_stop();
                guard.status.1 = PipelineState::Off;
                guard.last_error = Some(AUDIO_DIRECT_LOST_MESSAGE.to_string());
                (SideTeardown::None, capture, false)
            }
            _ => (SideTeardown::None, None, false),
        }
    };

    // Step 2 (no lock): bridge abort / TTS joins / capture join.
    teardown.run().await;
    join_optional_capture(capture).await;

    // Step 3 (re-lock): resume standby + publish. State may have moved during
    // teardown; resume/start_direct re-validate against current state.
    {
        let mut guard = engine.lock().await;
        if was_translate {
            match dir {
                Direction::Outbound => {
                    guard.resume_direct_outbound_if_enabled(config, devices, app);
                }
                Direction::Inbound => {
                    guard.resume_direct_inbound_if_enabled(config, devices, app);
                }
            }
            if !config.keep_direct_audio {
                match dir {
                    Direction::Outbound => guard.status.0 = PipelineState::Error,
                    Direction::Inbound => guard.status.1 = PipelineState::Error,
                }
            }
        }
        guard.publish_state(app);
    }
    end_live_meeting_after_audio_reconnect_exhausted_shared(engine, config, app).await;
}

async fn end_live_meeting_after_audio_reconnect_exhausted_shared(
    engine: &SharedEngine,
    config: &AppConfig,
    app: &AppHandle,
) {
    use crate::meeting::{end_active_meeting, ActiveMeetingId, MeetingStore};
    use tauri::Manager;

    let Some(active) = app.try_state::<ActiveMeetingId>() else {
        return;
    };
    let has_active = active
        .lock()
        .ok()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    if !has_active {
        return;
    }
    let Some(store) = app.try_state::<std::sync::Arc<MeetingStore>>() else {
        return;
    };

    tracing::info!(
        attempts = MAX_AUDIO_RECONNECT_ATTEMPTS,
        "ending live meeting after audio reconnect exhausted"
    );
    if let Err(error) = end_active_meeting(app, store.inner(), active.inner()) {
        tracing::warn!("failed to end meeting after audio reconnect exhausted: {error:#}");
        return;
    }

    // Return to Direct standby without holding the engine lock across joins.
    for dir in [Direction::Outbound, Direction::Inbound] {
        if let Err(e) = apply_audio_path_to_direct_shared(dir, engine, config, app).await {
            tracing::warn!(?dir, "return to direct standby failed: {e}");
        }
    }
}

pub(super) struct DirectEnsurePlan {
    pub join: Vec<CaptureHandle>,
    pub start_outbound: bool,
    pub start_inbound: bool,
    pub publish_after_standby_stop: bool,
}

impl TranslationEngine {
    /// Plan ensure/restart/stop of direct relays without joining capture threads.
    pub(super) fn plan_ensure_direct_audio(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
    ) -> DirectEnsurePlan {
        let mut join = Vec::new();
        let mut start_outbound = false;
        let mut start_inbound = false;
        let mut publish_after_standby_stop = false;

        if !config.keep_direct_audio {
            if !self.is_outbound_busy()
                && (self.outbound_side.direct.is_active() || self.status.0 == PipelineState::Direct)
            {
                if let Some(handle) = self.take_direct_outbound_for_stop() {
                    join.push(handle);
                }
                if self.status.0 == PipelineState::Direct {
                    self.status.0 = PipelineState::Off;
                    publish_after_standby_stop = true;
                }
            }
            if !self.is_inbound_busy()
                && (self.inbound_side.direct.is_active() || self.status.1 == PipelineState::Direct)
            {
                if let Some(handle) = self.take_direct_inbound_for_stop() {
                    join.push(handle);
                }
                if self.status.1 == PipelineState::Direct {
                    self.status.1 = PipelineState::Off;
                    publish_after_standby_stop = true;
                }
            }
            return DirectEnsurePlan {
                join,
                start_outbound,
                start_inbound,
                publish_after_standby_stop,
            };
        }

        if !self.is_outbound_busy()
            && config.validate_for_direct_outbound(devices).is_ok()
            && (self.status.0 == PipelineState::Off
                || (self.status.0 == PipelineState::Direct
                    && !self
                        .outbound_side
                        .direct
                        .is_outbound_healthy(config, devices)))
        {
            if self.outbound_side.direct.is_active() {
                if let Some(handle) = self.take_direct_outbound_for_stop() {
                    join.push(handle);
                }
                self.status.0 = PipelineState::Off;
                start_outbound = true;
            } else if self.status.0 != PipelineState::Direct {
                start_outbound = true;
            }
        }

        if !self.is_inbound_busy()
            && config.validate_for_direct_inbound(devices).is_ok()
            && (self.status.1 == PipelineState::Off
                || (self.status.1 == PipelineState::Direct
                    && !self.inbound_side.direct.is_inbound_healthy(config, devices)))
        {
            if self.inbound_side.direct.is_active() {
                if let Some(handle) = self.take_direct_inbound_for_stop() {
                    join.push(handle);
                }
                self.status.1 = PipelineState::Off;
                start_inbound = true;
            } else if self.status.1 != PipelineState::Direct {
                start_inbound = true;
            }
        }

        DirectEnsurePlan {
            join,
            start_outbound,
            start_inbound,
            publish_after_standby_stop,
        }
    }

    pub(super) fn apply_direct_ensure_starts(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
        plan: &DirectEnsurePlan,
    ) {
        if plan.publish_after_standby_stop {
            self.publish_state(app);
        }
        if plan.start_outbound && !self.is_outbound_busy() {
            let _ = self.start_direct_outbound(config, devices, app);
        }
        if plan.start_inbound && !self.is_inbound_busy() {
            let _ = self.start_direct_inbound(config, devices, app);
        }
    }
}

pub async fn ensure_direct_audio_shared(
    engine: SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
) -> Result<(), String> {
    if !config.keep_direct_audio {
        let plan = {
            let mut guard = engine.lock().await;
            guard.plan_ensure_direct_audio(config, devices)
        };
        let DirectEnsurePlan {
            join,
            start_outbound,
            start_inbound,
            publish_after_standby_stop,
        } = plan;
        for handle in join {
            join_capture_handle(handle).await;
        }
        let mut guard = engine.lock().await;
        guard.apply_direct_ensure_starts(
            config,
            devices,
            app,
            &DirectEnsurePlan {
                join: Vec::new(),
                start_outbound,
                start_inbound,
                publish_after_standby_stop,
            },
        );
        return Ok(());
    }

    {
        let mut guard = engine.lock().await;
        guard.try_recover_lost_audio_paths(config, devices);
    }

    let outbound_action = {
        let mut guard = engine.lock().await;
        if guard.is_outbound_busy() || config.validate_for_direct_outbound(devices).is_err() {
            EnsureAction::None
        } else if guard
            .outbound_side
            .direct
            .is_outbound_healthy(config, devices)
        {
            if !guard.outbound_side.audio_path.is_ok() {
                guard.clear_outbound_audio_state();
            }
            EnsureAction::None
        } else if guard.outbound_side.direct.is_active() || guard.status.0 == PipelineState::Direct
        {
            let handle = guard.take_direct_outbound_for_stop();
            guard.status.0 = PipelineState::Off;
            EnsureAction::Restart(handle)
        } else {
            EnsureAction::Start
        }
    };
    match outbound_action {
        EnsureAction::None => {}
        EnsureAction::Start => {
            let mut guard = engine.lock().await;
            if !guard.is_outbound_busy() {
                guard.start_direct_outbound(config, devices, app)?;
            }
        }
        EnsureAction::Restart(handle) => {
            join_optional_capture(handle).await;
            let mut guard = engine.lock().await;
            if !guard.is_outbound_busy() {
                guard.start_direct_outbound(config, devices, app)?;
            }
        }
    }

    let inbound_action = {
        let mut guard = engine.lock().await;
        if guard.is_inbound_busy() || config.validate_for_direct_inbound(devices).is_err() {
            EnsureAction::None
        } else if guard
            .inbound_side
            .direct
            .is_inbound_healthy(config, devices)
        {
            if !guard.inbound_side.audio_path.is_ok() {
                guard.clear_inbound_audio_state();
            }
            EnsureAction::None
        } else if guard.inbound_side.direct.is_active() || guard.status.1 == PipelineState::Direct {
            let handle = guard.take_direct_inbound_for_stop();
            guard.status.1 = PipelineState::Off;
            EnsureAction::Restart(handle)
        } else {
            EnsureAction::Start
        }
    };
    match inbound_action {
        EnsureAction::None => {}
        EnsureAction::Start => {
            let mut guard = engine.lock().await;
            if !guard.is_inbound_busy() {
                guard.start_direct_inbound(config, devices, app)?;
            }
        }
        EnsureAction::Restart(handle) => {
            join_optional_capture(handle).await;
            let mut guard = engine.lock().await;
            if !guard.is_inbound_busy() {
                guard.start_direct_inbound(config, devices, app)?;
            }
        }
    }

    {
        let mut guard = engine.lock().await;
        guard.ensure_watchdog(app.clone(), engine.clone());
    }
    Ok(())
}

enum EnsureAction {
    None,
    Start,
    Restart(Option<CaptureHandle>),
}

/// Full outbound stop without holding the engine lock across bridge aborts /
/// TTS worker joins / capture join (IPC- and watchdog-safe variant).
pub async fn stop_outbound_shared(engine: &SharedEngine, config: &AppConfig, app: &AppHandle) {
    let (parts, capture) = {
        let mut guard = engine.lock().await;
        guard.reset_mic_mute();
        guard.outbound_side.starting = false;
        if guard.status.0 != PipelineState::Stopping {
            guard.status.0 = PipelineState::Stopping;
            guard.publish_state(app);
        }
        // Persist trailing live text immediately — do not wait on provider close.
        guard.flush_live_transcript_segments(app, &["outbound"]);
        guard.cancel_all_outbound_tasks();
        let (parts, capture) = guard.outbound.take_teardown_parts();
        guard.outbound_side.active_since = None;
        guard.clear_bridge_state(Direction::Outbound);
        if guard.status.1 != PipelineState::Error {
            guard.last_error = None;
        }
        guard.clear_transcript_relay_if_idle();
        (parts, capture)
    };
    teardown_outbound_parts(parts).await;
    join_optional_capture(capture).await;
    let devices = crate::audio::list_devices_async().await.unwrap_or_default();
    let mut guard = engine.lock().await;
    guard.resume_direct_outbound_if_enabled(config, &devices, app);
}

/// Full inbound stop without holding the engine lock across bridge aborts /
/// TTS worker joins / capture join (IPC- and watchdog-safe variant).
pub async fn stop_inbound_shared(engine: &SharedEngine, config: &AppConfig, app: &AppHandle) {
    let (parts, capture) = {
        let mut guard = engine.lock().await;
        guard.reset_speaker_mute();
        guard.inbound_side.starting = false;
        if guard.status.1 != PipelineState::Stopping {
            guard.status.1 = PipelineState::Stopping;
            guard.publish_state(app);
        }
        // Persist trailing live text immediately — do not wait on provider close.
        guard.flush_live_transcript_segments(app, &["inbound"]);
        guard.cancel_all_inbound_tasks();
        let (parts, capture) = guard.inbound.take_teardown_parts();
        guard.inbound_side.active_since = None;
        guard.clear_bridge_state(Direction::Inbound);
        if guard.status.0 != PipelineState::Error {
            guard.last_error = None;
        }
        guard.clear_transcript_relay_if_idle();
        (parts, capture)
    };
    teardown_inbound_parts(parts).await;
    join_optional_capture(capture).await;
    let devices = crate::audio::list_devices_async().await.unwrap_or_default();
    let mut guard = engine.lock().await;
    guard.resume_direct_inbound_if_enabled(config, &devices, app);
}

/// `apply_*_audio_path(AudioPathMode::Direct)` semantics without lock-held
/// joins: classify under a short lock, tear down outside, start/resume after
/// re-locking (start_direct_* re-validate busy state themselves).
pub async fn apply_audio_path_to_direct_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    app: &AppHandle,
) -> Result<(), String> {
    enum ApplyDirectAction {
        AlreadyDirect,
        StopTranslate,
        StartOrRestart(Option<CaptureHandle>),
    }

    let action = {
        let mut guard = engine.lock().await;
        let busy = match dir {
            Direction::Outbound => guard.is_outbound_busy(),
            Direction::Inbound => guard.is_inbound_busy(),
        };
        if busy {
            ApplyDirectAction::StopTranslate
        } else {
            let status = match dir {
                Direction::Outbound => guard.status.0.clone(),
                Direction::Inbound => guard.status.1.clone(),
            };
            if status == PipelineState::Direct {
                ApplyDirectAction::AlreadyDirect
            } else {
                let handle = match dir {
                    Direction::Outbound => guard.take_direct_outbound_for_stop(),
                    Direction::Inbound => guard.take_direct_inbound_for_stop(),
                };
                ApplyDirectAction::StartOrRestart(handle)
            }
        }
    };

    match action {
        ApplyDirectAction::AlreadyDirect => Ok(()),
        ApplyDirectAction::StopTranslate => {
            match dir {
                Direction::Outbound => stop_outbound_shared(engine, config, app).await,
                Direction::Inbound => stop_inbound_shared(engine, config, app).await,
            }
            Ok(())
        }
        ApplyDirectAction::StartOrRestart(handle) => {
            join_optional_capture(handle).await;
            let devices = crate::audio::list_devices_async()
                .await
                .map_err(|e| e.to_string())?;
            let mut guard = engine.lock().await;
            match dir {
                Direction::Outbound => guard.start_direct_outbound(config, &devices, app),
                Direction::Inbound => guard.start_direct_inbound(config, &devices, app),
            }
        }
    }
}

pub(super) async fn maybe_ensure_direct_audio_shared(
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
) {
    let plan = {
        let mut guard = engine.lock().await;
        guard.plan_ensure_direct_audio(config, devices)
    };
    let DirectEnsurePlan {
        join,
        start_outbound,
        start_inbound,
        publish_after_standby_stop,
    } = plan;
    for handle in join {
        join_capture_handle(handle).await;
    }
    let mut guard = engine.lock().await;
    guard.apply_direct_ensure_starts(
        config,
        devices,
        app,
        &DirectEnsurePlan {
            join: Vec::new(),
            start_outbound,
            start_inbound,
            publish_after_standby_stop,
        },
    );
}

pub(super) async fn check_audio_health_shared(
    engine: &SharedEngine,
    app: &AppHandle,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) {
    use super::{AUDIO_HEARTBEAT_STALE_MS, AUDIO_STARTUP_GRACE_MS};

    if !config.keep_direct_audio {
        let plan = {
            let mut guard = engine.lock().await;
            guard.plan_ensure_direct_audio(config, devices)
        };
        let DirectEnsurePlan {
            join,
            start_outbound,
            start_inbound,
            publish_after_standby_stop,
        } = plan;
        for handle in join {
            join_capture_handle(handle).await;
        }
        let mut guard = engine.lock().await;
        guard.apply_direct_ensure_starts(
            config,
            devices,
            app,
            &DirectEnsurePlan {
                join: Vec::new(),
                start_outbound,
                start_inbound,
                publish_after_standby_stop,
            },
        );
        return;
    }

    let now_ms = unix_ms_now_u64();
    let (out_action, in_action) = {
        let mut guard = engine.lock().await;
        guard.watchdog_tick_count = guard.watchdog_tick_count.saturating_add(1);

        // A just-started relay churns the endpoint list itself (BT HFP profile
        // switch), and an unstable catalog must never condemn a running relay.
        let catalog_stable = guard.catalog_stable(now_ms);
        let in_startup_grace = |started_at_ms: Option<u64>| {
            started_at_ms
                .is_some_and(|started| now_ms.saturating_sub(started) < AUDIO_STARTUP_GRACE_MS)
        };

        let out_action = if guard.outbound_side.audio_path.is_reconnecting() {
            HealthAction::Supervise
        } else if guard.status.0 == PipelineState::Direct && guard.outbound_side.direct.is_active()
        {
            if !catalog_stable || in_startup_grace(guard.outbound_side.direct.started_at_ms()) {
                HealthAction::None
            } else {
                let stale = guard.outbound_side.direct.heartbeat().is_stale(
                    now_ms,
                    AUDIO_HEARTBEAT_STALE_MS,
                    guard.outbound_side.direct.started_at_ms(),
                    AUDIO_STARTUP_GRACE_MS,
                );
                if stale
                    || !guard
                        .outbound_side
                        .direct
                        .is_outbound_healthy(config, devices)
                {
                    HealthAction::Recover
                } else {
                    HealthAction::None
                }
            }
        } else if guard.status.0 == PipelineState::Active && guard.outbound.is_active() {
            if !catalog_stable || in_startup_grace(guard.outbound.started_at_ms()) {
                HealthAction::None
            } else {
                let stale = guard.outbound.heartbeat().is_stale(
                    now_ms,
                    AUDIO_HEARTBEAT_STALE_MS,
                    guard.outbound.started_at_ms(),
                    AUDIO_STARTUP_GRACE_MS,
                );
                if stale || !guard.outbound.is_audio_path_healthy(config, devices) {
                    HealthAction::Recover
                } else {
                    HealthAction::None
                }
            }
        } else {
            HealthAction::None
        };

        let in_action = if guard.inbound_side.audio_path.is_reconnecting() {
            HealthAction::Supervise
        } else if guard.status.1 == PipelineState::Direct && guard.inbound_side.direct.is_active() {
            if !catalog_stable || in_startup_grace(guard.inbound_side.direct.started_at_ms()) {
                HealthAction::None
            } else {
                let stale = guard.inbound_side.direct.heartbeat().is_stale(
                    now_ms,
                    AUDIO_HEARTBEAT_STALE_MS,
                    guard.inbound_side.direct.started_at_ms(),
                    AUDIO_STARTUP_GRACE_MS,
                );
                if stale
                    || !guard
                        .inbound_side
                        .direct
                        .is_inbound_healthy(config, devices)
                {
                    HealthAction::Recover
                } else {
                    HealthAction::None
                }
            }
        } else if guard.status.1 == PipelineState::Active && guard.inbound.is_active() {
            if !catalog_stable || in_startup_grace(guard.inbound.started_at_ms()) {
                HealthAction::None
            } else {
                let stale = guard.inbound.heartbeat().is_stale(
                    now_ms,
                    AUDIO_HEARTBEAT_STALE_MS,
                    guard.inbound.started_at_ms(),
                    AUDIO_STARTUP_GRACE_MS,
                );
                if stale || !guard.inbound.is_audio_path_healthy(config, devices) {
                    HealthAction::Recover
                } else {
                    HealthAction::None
                }
            }
        } else {
            HealthAction::None
        };

        (out_action, in_action)
    };

    match out_action {
        HealthAction::None => {}
        HealthAction::Supervise => {
            supervise_audio_recovery_shared(
                Direction::Outbound,
                engine,
                config,
                devices,
                app,
                now_ms,
            )
            .await;
        }
        HealthAction::Recover => {
            begin_audio_recovery_shared(Direction::Outbound, engine, config, devices, app).await;
        }
    }
    match in_action {
        HealthAction::None => {}
        HealthAction::Supervise => {
            supervise_audio_recovery_shared(
                Direction::Inbound,
                engine,
                config,
                devices,
                app,
                now_ms,
            )
            .await;
        }
        HealthAction::Recover => {
            begin_audio_recovery_shared(Direction::Inbound, engine, config, devices, app).await;
        }
    }
}

enum HealthAction {
    None,
    Supervise,
    Recover,
}

async fn supervise_audio_recovery_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
    now_ms: u64,
) {
    let should_recover = {
        let mut guard = engine.lock().await;
        if !guard.side(dir).audio_path.is_reconnecting() {
            return;
        }
        if guard.audio_recovery_confirmed(dir, config, devices, now_ms) {
            guard.clear_audio_state(dir);
            guard.publish_state(app);
            return;
        }
        if !guard.catalog_stable(now_ms) {
            // Catalog moved again after this attempt was scheduled — push the
            // attempt out to the new settle point rather than restarting
            // against a mid-churn enumeration.
            if let Some(stable_at) = guard.catalog_stable_at() {
                guard.side_mut(dir).audio_path.set_next_action_ms(stable_at);
            }
            return;
        }
        if !guard.side(dir).audio_path.next_action_due(now_ms) {
            return;
        }
        guard.audio_still_unhealthy(dir, config, devices, now_ms)
    };
    if should_recover {
        begin_audio_recovery_shared(dir, engine, config, devices, app).await;
    }
}

pub(super) async fn handle_audio_fault_shared(
    dir: Direction,
    engine: &SharedEngine,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    app: &AppHandle,
) {
    let defer_until = {
        let guard = engine.lock().await;
        let is_active = match dir {
            Direction::Outbound => guard.is_outbound_audio_active(),
            Direction::Inbound => guard.is_inbound_audio_active(),
        };
        if !is_active || guard.side(dir).audio_path.is_lost() {
            return;
        }
        if guard.side(dir).audio_path.is_reconnecting() {
            return;
        }
        let now_ms = unix_ms_now_u64();
        if guard.catalog_stable(now_ms) {
            None
        } else {
            guard.catalog_stable_at()
        }
    };
    if let Some(stable_at) = defer_until {
        // Endpoint churn (BT profile switch, virtual-driver engine restart)
        // faults streams and scrambles enumeration at the same time. Defer the
        // restart to the settle point instead of failing it against a
        // mid-churn catalog; the watchdog supervisor runs it from there.
        let capture = {
            let mut guard = engine.lock().await;
            // The faulting worker is already dead/dying: take a direct relay
            // down now so its stale handle cannot be falsely "confirmed"
            // healthy during the defer window. Translate pipelines keep their
            // own recovery flow and are left untouched here.
            let handle = match dir {
                Direction::Outbound if guard.status.0 == PipelineState::Direct => {
                    let handle = guard.take_direct_outbound_for_stop();
                    guard.status.0 = PipelineState::Off;
                    handle
                }
                Direction::Inbound if guard.status.1 == PipelineState::Direct => {
                    let handle = guard.take_direct_inbound_for_stop();
                    guard.status.1 = PipelineState::Off;
                    handle
                }
                _ => None,
            };
            guard.side_mut(dir).audio_path.begin_reconnect(1);
            guard.side_mut(dir).audio_path.set_next_action_ms(stable_at);
            // The ensure path performs the actual restart once the catalog
            // settles — make sure it runs even off the periodic cadence.
            guard.ensure_direct_audio_pending = true;
            guard.publish_state(app);
            handle
        };
        join_optional_capture(capture).await;
        return;
    }
    begin_audio_recovery_shared(dir, engine, config, devices, app).await;
}

pub(super) async fn check_proactive_session_refresh_shared(
    engine: &SharedEngine,
    app: &AppHandle,
    config: &AppConfig,
) {
    use super::{
        run_finish_start_inbound, run_finish_start_outbound, unix_ms_now, PROACTIVE_SESSION_MS,
    };
    use tauri::Manager;
    use tokio_util::sync::CancellationToken;

    if !config.proactive_session_refresh {
        return;
    }

    let now = unix_ms_now();
    let outbound_teardown = {
        let mut guard = engine.lock().await;
        if guard.status.0 == PipelineState::Active {
            if let Some(since) = guard.outbound_side.active_since {
                if now - since >= PROACTIVE_SESSION_MS {
                    tracing::info!("proactive outbound session refresh after 60 minutes");
                    guard.outbound_side.active_since = Some(now);
                    guard.flush_live_transcript_segments(app, &["outbound"]);
                    guard.cancel_all_outbound_tasks();
                    let (parts, capture) = guard.outbound.take_teardown_parts();
                    guard.outbound_side.starting = true;
                    guard.outbound_side.pending_cancel = Some(CancellationToken::new());
                    guard.status.0 = PipelineState::Starting;
                    guard.clear_bridge_state(Direction::Outbound);
                    if let Some(seg) =
                        app.try_state::<std::sync::Arc<crate::meeting::SharedSegmentEngine>>()
                    {
                        seg.open_direction("outbound");
                    }
                    guard.publish_state(app);
                    Some((parts, capture))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };
    if let Some((parts, capture)) = outbound_teardown {
        teardown_outbound_parts(parts).await;
        join_optional_capture(capture).await;
        let engine_bg = engine.clone();
        let app_bg = app.clone();
        let config_bg = config.clone();
        tokio::spawn(async move {
            run_finish_start_outbound(engine_bg, config_bg, app_bg).await;
        });
    }

    let inbound_teardown = {
        let mut guard = engine.lock().await;
        if guard.status.1 == PipelineState::Active {
            if let Some(since) = guard.inbound_side.active_since {
                if now - since >= PROACTIVE_SESSION_MS {
                    tracing::info!("proactive inbound session refresh after 60 minutes");
                    guard.inbound_side.active_since = Some(now);
                    guard.flush_live_transcript_segments(app, &["inbound"]);
                    guard.cancel_all_inbound_tasks();
                    let (parts, capture) = guard.inbound.take_teardown_parts();
                    guard.inbound_side.starting = true;
                    guard.inbound_side.pending_cancel = Some(CancellationToken::new());
                    guard.status.1 = PipelineState::Starting;
                    guard.clear_bridge_state(Direction::Inbound);
                    if let Some(seg) =
                        app.try_state::<std::sync::Arc<crate::meeting::SharedSegmentEngine>>()
                    {
                        seg.open_direction("inbound");
                    }
                    guard.publish_state(app);
                    Some((parts, capture))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };
    if let Some((parts, capture)) = inbound_teardown {
        teardown_inbound_parts(parts).await;
        join_optional_capture(capture).await;
        let engine_bg = engine.clone();
        let app_bg = app.clone();
        let config_bg = config.clone();
        tokio::spawn(async move {
            run_finish_start_inbound(engine_bg, config_bg, app_bg).await;
        });
    }
}

pub(super) async fn check_meeting_idle_end_shared(
    engine: &SharedEngine,
    app: &AppHandle,
    config: &AppConfig,
) {
    use super::MEETING_IDLE_NO_SEGMENT_MS;
    use crate::meeting::{
        clear_meeting_idle_clock, end_active_meeting, meeting_idle_elapsed_ms, ActiveMeetingId,
        MeetingIdleClock, MeetingStore,
    };
    use tauri::Manager;

    let Some(clock) = app.try_state::<MeetingIdleClock>() else {
        return;
    };
    let now = unix_ms_now_u64();
    let Some(elapsed) = meeting_idle_elapsed_ms(clock.inner(), now) else {
        return;
    };
    if elapsed < MEETING_IDLE_NO_SEGMENT_MS {
        return;
    }

    let Some(active) = app.try_state::<ActiveMeetingId>() else {
        return;
    };
    let has_active = active
        .lock()
        .ok()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    if !has_active {
        clear_meeting_idle_clock(clock.inner());
        return;
    }

    let Some(store) = app.try_state::<std::sync::Arc<MeetingStore>>() else {
        return;
    };

    tracing::info!(
        elapsed_ms = elapsed,
        "ending meeting after {MEETING_IDLE_NO_SEGMENT_MS}ms with no new segments"
    );
    if let Err(error) = end_active_meeting(app, store.inner(), active.inner()) {
        tracing::warn!("failed to end idle meeting: {error:#}");
        return;
    }

    // Return to Direct standby without holding the engine lock across joins.
    for dir in [Direction::Outbound, Direction::Inbound] {
        if let Err(e) = apply_audio_path_to_direct_shared(dir, engine, config, app).await {
            tracing::warn!(?dir, "return to direct standby failed: {e}");
        }
    }
}
