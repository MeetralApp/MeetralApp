use super::{
    config_from_state, format_connection_gap_stamp, unix_ms_now, BridgeConnectionState, Direction,
    PipelineState, SharedEngine, TranslationEngine, BRIDGE_FATAL_USER_MESSAGE,
};
use crate::ai::{BridgeStatusEvent, TranscriptEvent};
use crate::audio::list_devices_async;
use crate::voice::types::{VoiceCloneLatencyEvent, VoiceTtsStatus, VoiceTtsStatusPayload};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

async fn handle_bridge_fatal_shared(
    direction: Direction,
    engine: SharedEngine,
    detail: String,
    app: AppHandle,
) {
    // Step 1 (short lock): mutate state, take teardown parts.
    let (teardown, capture) = {
        let mut guard = engine.lock().await;
        match direction {
            Direction::Outbound => {
                if guard.status.0 != PipelineState::Active
                    && guard.status.0 != PipelineState::Starting
                {
                    return;
                }
                tracing::error!("outbound bridge fatal: {detail}");
                guard.reset_mic_mute();
                guard.flush_live_transcript_segments(&app, &["outbound"]);
                guard.cancel_all_outbound_tasks();
                guard.side_mut(direction).starting = false;
                let (parts, capture) = guard.outbound.take_teardown_parts();
                guard.side_mut(direction).active_since = None;
                guard.clear_bridge_state(direction);
                guard.last_error = Some(format!("{BRIDGE_FATAL_USER_MESSAGE} ({detail})"));
                guard.clear_transcript_relay_if_idle();
                (super::lock_scope::SideTeardown::Outbound(parts), capture)
            }
            Direction::Inbound => {
                if guard.status.1 != PipelineState::Active {
                    return;
                }
                tracing::error!("inbound bridge fatal: {detail}");
                guard.reset_speaker_mute();
                guard.flush_live_transcript_segments(&app, &["inbound"]);
                if let Some(cancel) = guard.side_mut(direction).cancel.take() {
                    cancel.cancel();
                }
                let (parts, capture) = guard.inbound.take_teardown_parts();
                guard.side_mut(direction).active_since = None;
                guard.clear_bridge_state(direction);
                guard.last_error = Some(format!("{BRIDGE_FATAL_USER_MESSAGE} ({detail})"));
                guard.clear_transcript_relay_if_idle();
                (super::lock_scope::SideTeardown::Inbound(parts), capture)
            }
        }
    };

    // Step 2 (no lock): bridge abort / TTS joins / capture join.
    teardown.run().await;
    super::lock_scope::join_optional_capture(capture).await;

    // Step 3 (re-lock): resume standby + publish.
    let config = config_from_state(&app).await;
    let devices = list_devices_async().await.unwrap_or_default();
    let mut guard = engine.lock().await;
    match direction {
        Direction::Outbound => {
            guard.resume_direct_outbound_if_enabled(&config, &devices, &app);
            if !config.keep_direct_audio {
                guard.status.0 = PipelineState::Error;
            }
        }
        Direction::Inbound => {
            guard.resume_direct_inbound_if_enabled(&config, &devices, &app);
            if !config.keep_direct_audio {
                guard.status.1 = PipelineState::Error;
            }
        }
    }
    guard.publish_state(&app);
}

impl TranslationEngine {
    pub(super) fn spawn_fatal_listener(
        direction: Direction,
        engine: SharedEngine,
        app: AppHandle,
        mut fatal_rx: mpsc::Receiver<String>,
    ) {
        tokio::spawn(async move {
            if let Some(detail) = fatal_rx.recv().await {
                handle_bridge_fatal_shared(direction, engine, detail, app).await;
            }
        });
    }

    pub(super) fn spawn_bridge_status_listener(
        engine: SharedEngine,
        app: AppHandle,
        mut status_rx: mpsc::Receiver<BridgeStatusEvent>,
    ) {
        tokio::spawn(async move {
            while let Some(event) = status_rx.recv().await {
                let mut guard = engine.lock().await;
                guard.handle_bridge_status_event(event, &app);
            }
        });
    }

    pub(crate) fn apply_bridge_status_event(&mut self, event: BridgeStatusEvent) -> bool {
        match event {
            BridgeStatusEvent::Reconnecting { direction, attempt } => {
                self.bridge_reconnect_count = self.bridge_reconnect_count.saturating_add(1);
                let Ok(direction) = Direction::try_from(direction.as_str()) else {
                    return false;
                };
                let is_active = match direction {
                    Direction::Outbound => self.status.0 == PipelineState::Active,
                    Direction::Inbound => self.status.1 == PipelineState::Active,
                };
                if is_active {
                    let side = self.side_mut(direction);
                    side.bridge = BridgeConnectionState::Reconnecting;
                    side.reconnect_attempt = Some(attempt);
                    return true;
                }
            }
            BridgeStatusEvent::Ready {
                direction,
                reconnected,
            } => {
                let Ok(direction) = Direction::try_from(direction.as_str()) else {
                    return false;
                };
                let is_active = match direction {
                    Direction::Outbound => self.status.0 == PipelineState::Active,
                    Direction::Inbound => self.status.1 == PipelineState::Active,
                };
                if is_active {
                    let side = self.side_mut(direction);
                    side.bridge = BridgeConnectionState::Ready;
                    side.reconnect_attempt = None;
                    if reconnected {
                        side.active_since = Some(unix_ms_now());
                    }
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn handle_bridge_status_event(&mut self, event: BridgeStatusEvent, app: &AppHandle) {
        let gap_direction = match &event {
            BridgeStatusEvent::Reconnecting { direction, .. } => {
                match Direction::try_from(direction.as_str()) {
                    Ok(direction) => {
                        let was_ready = self.side(direction).bridge == BridgeConnectionState::Ready;
                        was_ready.then(|| direction.as_str().to_string())
                    }
                    Err(()) => None,
                }
            }
            _ => None,
        };
        if self.apply_bridge_status_event(event) {
            if let Some(direction) = gap_direction {
                self.emit_connection_gap(&direction, app);
            }
            self.publish_state(app);
        }
    }

    pub(super) fn emit_connection_gap(&self, direction: &str, app: &AppHandle) {
        let stamp = format_connection_gap_stamp();
        let label = format!("[Connection interrupted {stamp}]");
        let event = TranscriptEvent {
            direction: direction.to_string(),
            source_text: Some(label.clone()),
            translated_text: Some(label),
            interim: false,
            turn_complete: false,
            input_segment_finished: false,
            output_segment_finished: false,
            connection_gap: true,
            replace_live: false,
            live_source: None,
            live_translated: None,
        };
        if let Some(tx) = &self.transcript_tx {
            crate::runtime::control_channel::try_send_control(
                tx,
                event.clone(),
                "transcript-fanout",
            );
        } else {
            let _ = app.emit("transcript", &event);
        }
    }

    pub(super) fn clear_bridge_state(&mut self, direction: Direction) {
        let side = self.side_mut(direction);
        side.bridge = BridgeConnectionState::Idle;
        side.reconnect_attempt = None;
    }

    pub(super) fn spawn_voice_tts_status_listener(
        app: AppHandle,
        mut status_rx: mpsc::Receiver<VoiceTtsStatus>,
    ) {
        tokio::spawn(async move {
            while let Some(status) = status_rx.recv().await {
                let payload = VoiceTtsStatusPayload::from(status);
                let _ = app.emit("voice-tts-status", &payload);
            }
        });
    }

    pub(super) fn spawn_voice_clone_latency_listener(
        app: AppHandle,
        mut latency_rx: mpsc::Receiver<VoiceCloneLatencyEvent>,
    ) {
        tokio::spawn(async move {
            while let Some(event) = latency_rx.recv().await {
                let _ = app.emit("voice-clone-latency", &event);
            }
        });
    }
}
