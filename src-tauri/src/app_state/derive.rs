use crate::app_state::snapshot::{
    ColumnIdleBadgeSnapshot, ColumnUiSnapshot, ColumnUiState, SetupState,
};
use crate::audio::{validate_audio_setup, AudioDeviceInfo, AudioRole, AudioSetupValidation};
use crate::config::AppConfig;
use crate::runtime::engine::{
    AppStatus, AudioConnectionState, BridgeConnectionState, PipelineState,
};

/// Read-only view of engine + config used to derive all UI-facing state in one place.
#[derive(Debug, Clone)]
pub struct EngineView<'a> {
    pub config: &'a AppConfig,
    pub devices: &'a [AudioDeviceInfo],
    pub outbound_pipeline: PipelineState,
    pub inbound_pipeline: PipelineState,
    pub outbound_audio_raw: AudioConnectionState,
    pub inbound_audio_raw: AudioConnectionState,
    pub outbound_audio_attempt: Option<u32>,
    pub inbound_audio_attempt: Option<u32>,
    pub runtime_error: Option<String>,
    pub outbound_active_since: Option<i64>,
    pub inbound_active_since: Option<i64>,
    pub outbound_bridge: BridgeConnectionState,
    pub inbound_bridge: BridgeConnectionState,
    pub outbound_bridge_attempt: Option<u32>,
    pub inbound_bridge_attempt: Option<u32>,
    pub running: bool,
    pub mic_muted: bool,
    pub speaker_muted: bool,
}

impl<'a> EngineView<'a> {
    pub fn outbound_pipeline_live(&self) -> bool {
        matches!(
            self.outbound_pipeline,
            PipelineState::Direct
                | PipelineState::Starting
                | PipelineState::Stopping
                | PipelineState::Active
        )
    }

    pub fn inbound_pipeline_live(&self) -> bool {
        matches!(
            self.inbound_pipeline,
            PipelineState::Direct
                | PipelineState::Starting
                | PipelineState::Stopping
                | PipelineState::Active
        )
    }

    pub fn normalized_outbound_audio(&self) -> AudioConnectionState {
        normalize_column_audio(
            self.outbound_audio_raw.clone(),
            self.outbound_pipeline_live(),
        )
    }

    pub fn normalized_inbound_audio(&self) -> AudioConnectionState {
        normalize_column_audio(self.inbound_audio_raw.clone(), self.inbound_pipeline_live())
    }
}

fn normalize_column_audio(raw: AudioConnectionState, pipeline_live: bool) -> AudioConnectionState {
    if matches!(
        raw,
        AudioConnectionState::Reconnecting | AudioConnectionState::Lost
    ) {
        return raw;
    }
    if pipeline_live {
        raw
    } else {
        AudioConnectionState::Ok
    }
}

fn role_strictly_resolved(validation: &AudioSetupValidation, role: AudioRole) -> bool {
    validation
        .roles
        .iter()
        .find(|r| r.role == role)
        .is_some_and(|r| r.resolved)
}

/// Direct always needs capture + playback, even when Translate is text-only.
fn direct_path_strictly_ready(validation: &AudioSetupValidation, outbound: bool) -> bool {
    if outbound {
        role_strictly_resolved(validation, AudioRole::UserMic)
            && role_strictly_resolved(validation, AudioRole::TeamsMicFeed)
    } else {
        role_strictly_resolved(validation, AudioRole::MeetingCapture)
            && role_strictly_resolved(validation, AudioRole::LocalPlayback)
    }
}

fn can_start_outbound_audio(config: &AppConfig, validation: &AudioSetupValidation) -> bool {
    if !config.outbound_mode.needs_playback() {
        return validation
            .roles
            .iter()
            .find(|r| r.role == AudioRole::UserMic)
            .map(|r| r.resolved)
            .unwrap_or(true);
    }
    validation.outbound_ready
}

fn can_start_inbound_audio(config: &AppConfig, validation: &AudioSetupValidation) -> bool {
    if !config.inbound_mode.needs_playback() {
        return validation
            .roles
            .iter()
            .find(|r| r.role == AudioRole::MeetingCapture)
            .map(|r| r.resolved)
            .unwrap_or(false);
    }
    validation.inbound_ready
}

fn column_audio_fault(state: &AudioConnectionState) -> bool {
    matches!(
        state,
        AudioConnectionState::Reconnecting | AudioConnectionState::Lost
    )
}

fn resolve_column_idle_badge(
    pipeline_state: &PipelineState,
    audio_state: &AudioConnectionState,
    column_path_ready: bool,
    api_configured: bool,
    translate_audio_ready: bool,
    runtime_error: &Option<String>,
) -> ColumnIdleBadgeSnapshot {
    if matches!(pipeline_state, PipelineState::Error) {
        return ColumnIdleBadgeSnapshot {
            kind: "error".into(),
            label: "Error".into(),
            title: runtime_error
                .clone()
                .unwrap_or_else(|| "Pipeline error".into()),
        };
    }

    if matches!(audio_state, AudioConnectionState::Lost) {
        return ColumnIdleBadgeSnapshot {
            kind: "audio-lost".into(),
            label: "Audio lost".into(),
            title: "Audio device disconnected — check Settings".into(),
        };
    }

    if matches!(audio_state, AudioConnectionState::Reconnecting) {
        return ColumnIdleBadgeSnapshot {
            kind: "audio-reconnecting".into(),
            label: "Audio…".into(),
            title: "Audio device reconnecting".into(),
        };
    }

    if !column_path_ready {
        if !api_configured && translate_audio_ready {
            return ColumnIdleBadgeSnapshot {
                kind: "api-key".into(),
                label: "API key".into(),
                title: "Add API key in Settings to enable Translate".into(),
            };
        }
        return ColumnIdleBadgeSnapshot {
            kind: "setup".into(),
            label: "Setup".into(),
            title: "Complete audio setup in Settings".into(),
        };
    }

    if !api_configured {
        return ColumnIdleBadgeSnapshot {
            kind: "api-key".into(),
            label: "API key".into(),
            title: "Add API key in Settings to enable Translate".into(),
        };
    }

    let mut parts = Vec::new();
    parts.push("Direct available");
    if translate_audio_ready {
        parts.push("Translate available");
    }
    ColumnIdleBadgeSnapshot {
        kind: "ready".into(),
        label: "Ready".into(),
        title: parts.join(" · "),
    }
}

fn translate_disabled_reason(
    can_translate: bool,
    audio: &AudioConnectionState,
    api_configured: bool,
    translate_audio_ready: bool,
    elevenlabs_ready: bool,
) -> Option<String> {
    if can_translate {
        return None;
    }
    if matches!(audio, AudioConnectionState::Lost) {
        return Some("Audio device disconnected".into());
    }
    if matches!(audio, AudioConnectionState::Reconnecting) {
        return Some("Audio device reconnecting".into());
    }
    if !api_configured {
        return Some("API key required".into());
    }
    if !elevenlabs_ready {
        return Some("Complete voice clone setup in Settings".into());
    }
    if !translate_audio_ready {
        return Some("Complete audio setup in Settings".into());
    }
    None
}

pub fn build_setup_state(view: &EngineView<'_>, revision: u64) -> SetupState {
    let outbound_audio = view.normalized_outbound_audio();
    let inbound_audio = view.normalized_inbound_audio();
    let audio = validate_audio_setup(view.config, view.devices);
    let api_configured = view.config.is_api_key_configured();

    let direct_outbound_ready = direct_path_strictly_ready(&audio, true);
    let direct_inbound_ready = direct_path_strictly_ready(&audio, false);

    // Direct + Translate gates use strict device presence (same as idle Ready badge).
    // Transient “not found” softening used to keep Translate clickable on text-only
    // when only capture resolved — misleading when playback is missing.
    let can_direct_outbound = direct_outbound_ready;
    let can_direct_inbound = direct_inbound_ready;

    let can_translate_outbound = api_configured
        && view.config.validate_elevenlabs_setup().is_ok()
        && direct_outbound_ready
        && can_start_outbound_audio(view.config, &audio)
        && !column_audio_fault(&outbound_audio);
    let can_translate_inbound = api_configured
        && direct_inbound_ready
        && can_start_inbound_audio(view.config, &audio)
        && !column_audio_fault(&inbound_audio);

    let outbound_translate_audio_ready = can_start_outbound_audio(view.config, &audio);
    let inbound_translate_audio_ready = can_start_inbound_audio(view.config, &audio);

    SetupState {
        revision,
        api_key_configured: api_configured,
        can_direct_outbound,
        can_direct_inbound,
        can_translate_outbound,
        can_translate_inbound,
        outbound_idle_badge: resolve_column_idle_badge(
            &view.outbound_pipeline,
            &outbound_audio,
            direct_outbound_ready,
            api_configured,
            outbound_translate_audio_ready,
            &view.runtime_error,
        ),
        inbound_idle_badge: resolve_column_idle_badge(
            &view.inbound_pipeline,
            &inbound_audio,
            direct_inbound_ready,
            api_configured,
            inbound_translate_audio_ready,
            &view.runtime_error,
        ),
        audio,
    }
}

fn build_column_ui(
    view: &EngineView<'_>,
    direction_outbound: bool,
    setup: &SetupState,
) -> ColumnUiState {
    let (pipeline, audio, pipeline_live) = if direction_outbound {
        (
            view.outbound_pipeline.clone(),
            view.normalized_outbound_audio(),
            view.outbound_pipeline_live(),
        )
    } else {
        (
            view.inbound_pipeline.clone(),
            view.normalized_inbound_audio(),
            view.inbound_pipeline_live(),
        )
    };

    let (can_direct, can_translate, idle_badge, translate_audio_ready) = if direction_outbound {
        (
            setup.can_direct_outbound,
            setup.can_translate_outbound,
            setup.outbound_idle_badge.clone(),
            can_start_outbound_audio(view.config, &setup.audio),
        )
    } else {
        (
            setup.can_direct_inbound,
            setup.can_translate_inbound,
            setup.inbound_idle_badge.clone(),
            can_start_inbound_audio(view.config, &setup.audio),
        )
    };

    let elevenlabs_ready = !direction_outbound || view.config.validate_elevenlabs_setup().is_ok();

    ColumnUiState {
        pipeline,
        audio_connection: audio.clone(),
        can_direct,
        can_translate,
        translate_disabled_reason: translate_disabled_reason(
            can_translate,
            &audio,
            setup.api_key_configured,
            translate_audio_ready,
            elevenlabs_ready,
        ),
        idle_badge,
        pipeline_live,
        mute_enabled: pipeline_live,
    }
}

pub fn build_column_ui_snapshot(view: &EngineView<'_>, setup: &SetupState) -> ColumnUiSnapshot {
    ColumnUiSnapshot {
        outbound: build_column_ui(view, true, setup),
        inbound: build_column_ui(view, false, setup),
    }
}

pub fn build_app_status(view: &EngineView<'_>) -> AppStatus {
    let outbound_active_since = if view.outbound_pipeline == PipelineState::Active {
        view.outbound_active_since
    } else {
        None
    };
    let inbound_active_since = if view.inbound_pipeline == PipelineState::Active {
        view.inbound_active_since
    } else {
        None
    };

    let outbound_bridge = if view.outbound_pipeline == PipelineState::Active {
        view.outbound_bridge.clone()
    } else {
        BridgeConnectionState::Idle
    };
    let inbound_bridge = if view.inbound_pipeline == PipelineState::Active {
        view.inbound_bridge.clone()
    } else {
        BridgeConnectionState::Idle
    };

    let outbound_reconnect_attempt = if view.outbound_pipeline == PipelineState::Active {
        view.outbound_bridge_attempt
    } else {
        None
    };
    let inbound_reconnect_attempt = if view.inbound_pipeline == PipelineState::Active {
        view.inbound_bridge_attempt
    } else {
        None
    };

    let outbound_audio = view.normalized_outbound_audio();
    let inbound_audio = view.normalized_inbound_audio();

    let outbound_audio_reconnect_attempt =
        if column_audio_fault(&outbound_audio) || view.outbound_pipeline_live() {
            view.outbound_audio_attempt
        } else {
            None
        };
    let inbound_audio_reconnect_attempt =
        if column_audio_fault(&inbound_audio) || view.inbound_pipeline_live() {
            view.inbound_audio_attempt
        } else {
            None
        };

    AppStatus {
        running: view.running,
        outbound: view.outbound_pipeline.clone(),
        inbound: view.inbound_pipeline.clone(),
        error: view.runtime_error.clone(),
        outbound_active_since,
        inbound_active_since,
        outbound_bridge,
        inbound_bridge,
        outbound_reconnect_attempt,
        inbound_reconnect_attempt,
        outbound_audio,
        inbound_audio,
        outbound_audio_reconnect_attempt,
        inbound_audio_reconnect_attempt,
        mic_muted: view.mic_muted,
        speaker_muted: view.speaker_muted,
    }
}

pub fn devices_list_changed(previous: &[AudioDeviceInfo], next: &[AudioDeviceInfo]) -> bool {
    previous != next
}

pub fn setup_semantically_eq(a: &SetupState, b: &SetupState) -> bool {
    a.api_key_configured == b.api_key_configured
        && a.can_direct_outbound == b.can_direct_outbound
        && a.can_direct_inbound == b.can_direct_inbound
        && a.can_translate_outbound == b.can_translate_outbound
        && a.can_translate_inbound == b.can_translate_inbound
        && a.outbound_idle_badge == b.outbound_idle_badge
        && a.inbound_idle_badge == b.inbound_idle_badge
        && a.audio == b.audio
}

pub fn snapshot_semantically_eq(
    a: &crate::app_state::snapshot::AppSnapshot,
    b: &crate::app_state::snapshot::AppSnapshot,
) -> bool {
    a.runtime == b.runtime
        && setup_semantically_eq(&a.setup, &b.setup)
        && a.columns == b.columns
        && a.devices == b.devices
        && a.config.config == b.config.config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::test_support::{sample_config, sample_devices};

    fn sample_view<'a>(
        config: &'a AppConfig,
        devices: &'a [AudioDeviceInfo],
        outbound_audio: AudioConnectionState,
    ) -> EngineView<'a> {
        EngineView {
            config,
            devices,
            outbound_pipeline: PipelineState::Off,
            inbound_pipeline: PipelineState::Off,
            outbound_audio_raw: outbound_audio,
            inbound_audio_raw: AudioConnectionState::Ok,
            outbound_audio_attempt: None,
            inbound_audio_attempt: None,
            runtime_error: None,
            outbound_active_since: None,
            inbound_active_since: None,
            outbound_bridge: BridgeConnectionState::Idle,
            inbound_bridge: BridgeConnectionState::Idle,
            outbound_bridge_attempt: None,
            inbound_bridge_attempt: None,
            running: false,
            mic_muted: false,
            speaker_muted: false,
        }
    }

    #[test]
    fn mute_enabled_follows_pipeline_live_not_mute_state() {
        let config = sample_config();
        let devices = sample_devices();
        let mut view = sample_view(&config, &devices, AudioConnectionState::Ok);
        view.outbound_pipeline = PipelineState::Active;
        view.inbound_pipeline = PipelineState::Direct;
        view.mic_muted = false;
        view.speaker_muted = true;

        let setup = build_setup_state(&view, 1);
        let columns = build_column_ui_snapshot(&view, &setup);

        assert!(columns.outbound.pipeline_live);
        assert!(columns.outbound.mute_enabled);
        assert!(columns.inbound.pipeline_live);
        assert!(columns.inbound.mute_enabled);
    }

    #[test]
    fn setup_and_columns_agree_on_translate_when_audio_ok() {
        let config = sample_config();
        let devices = sample_devices();
        let view = sample_view(&config, &devices, AudioConnectionState::Ok);
        let setup = build_setup_state(&view, 1);
        let columns = build_column_ui_snapshot(&view, &setup);
        assert_eq!(setup.can_translate_outbound, columns.outbound.can_translate);
    }

    #[test]
    fn setup_and_columns_agree_on_audio_lost() {
        let config = sample_config();
        let devices = sample_devices();
        let view = sample_view(&config, &devices, AudioConnectionState::Lost);
        let setup = build_setup_state(&view, 1);
        let columns = build_column_ui_snapshot(&view, &setup);
        assert!(
            setup.can_direct_outbound,
            "direct should stay allowed when only translate is blocked by audio fault"
        );
        assert!(!setup.can_translate_outbound);
        assert!(!columns.outbound.can_translate);
        assert_eq!(columns.outbound.idle_badge.kind, "audio-lost");
        assert_eq!(setup.outbound_idle_badge.kind, "audio-lost");
    }

    #[test]
    fn text_only_inbound_badge_needs_playback_device_for_ready() {
        use crate::config::PipelineOutputMode;

        let mut config = sample_config();
        config.inbound_mode = PipelineOutputMode::TextOnly;
        config.outbound_mode = PipelineOutputMode::TextOnly;
        // Capture present; headphones missing — Translate text-only may be ready,
        // but Direct still needs local playback so idle badge must stay setup.
        let devices: Vec<AudioDeviceInfo> = sample_devices()
            .into_iter()
            .filter(|d| d.id != "hp")
            .collect();
        let view = sample_view(&config, &devices, AudioConnectionState::Ok);
        let setup = build_setup_state(&view, 1);

        assert!(
            setup.audio.inbound_ready,
            "text-only inbound_ready should ignore missing playback"
        );
        assert!(!setup.can_direct_inbound);
        assert!(!setup.can_translate_inbound);
        assert_eq!(
            setup.inbound_idle_badge.kind, "setup",
            "Ready badge must require Direct path devices, not text-only readiness"
        );
    }

    #[test]
    fn runtime_and_setup_share_normalized_audio_when_direct_live() {
        let config = sample_config();
        let devices = sample_devices();
        let mut view = sample_view(&config, &devices, AudioConnectionState::Ok);
        view.outbound_pipeline = PipelineState::Direct;
        let setup = build_setup_state(&view, 1);
        let runtime = build_app_status(&view);
        assert_eq!(runtime.outbound_audio, AudioConnectionState::Ok);
        assert!(setup.can_translate_outbound);
    }
}
