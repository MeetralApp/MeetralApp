use super::types::DiagnosticsSnapshot;
use super::TranslationEngine;
use tauri::AppHandle;

impl TranslationEngine {
    pub fn set_mic_muted(&mut self, muted: bool, app: &AppHandle) -> Result<(), String> {
        if !self.is_outbound_audio_active() {
            return Err("Outbound audio is not active".into());
        }
        self.mic_mute.set_muted(muted);
        self.publish(app);
        Ok(())
    }

    pub fn set_speaker_muted(&mut self, muted: bool, app: &AppHandle) -> Result<(), String> {
        if !self.is_inbound_audio_active() {
            return Err("Inbound audio is not active".into());
        }
        self.speaker_mute.set_muted(muted);
        self.publish(app);
        Ok(())
    }

    pub fn diagnostics_snapshot(&self) -> DiagnosticsSnapshot {
        let outbound_capture_last_ms = if self.outbound_side.direct.is_active() {
            Some(self.outbound_side.direct.heartbeat().last_frame_ms())
        } else if self.outbound.is_active() {
            Some(self.outbound.heartbeat().last_frame_ms())
        } else {
            None
        };
        let inbound_capture_last_ms = if self.inbound_side.direct.is_active() {
            Some(self.inbound_side.direct.heartbeat().last_frame_ms())
        } else if self.inbound.is_active() {
            Some(self.inbound.heartbeat().last_frame_ms())
        } else {
            None
        };

        DiagnosticsSnapshot {
            outbound_state: format!("{:?}", self.status.0),
            inbound_state: format!("{:?}", self.status.1),
            outbound_capture_last_ms,
            inbound_capture_last_ms,
            frames_dropped: self
                .pcm_frames_dropped
                .load(std::sync::atomic::Ordering::Relaxed)
                .saturating_add(
                    self.transcript_db_metrics
                        .dropped_events
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
            db_write_queue_depth: self
                .transcript_db_metrics
                .queue_depth
                .load(std::sync::atomic::Ordering::Relaxed),
            reconnect_count: self.bridge_reconnect_count,
            panics_since_start: crate::error::panic_count(),
            control_channel_drops: crate::runtime::control_channel::control_channel_drops(),
        }
    }
}
