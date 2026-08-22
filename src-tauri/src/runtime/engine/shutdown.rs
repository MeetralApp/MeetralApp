use super::types::PipelineState;
use super::{Direction, TranslationEngine};
use crate::meeting::{end_active_meeting, ActiveMeetingId, MeetingStore};
use tauri::{AppHandle, Manager};

impl TranslationEngine {
    pub async fn graceful_shutdown(&mut self, app: &AppHandle) {
        // Stop pipelines first so live transcript is flushed while active meeting still set.
        self.stop_outbound_for_quit(app).await;
        self.stop_inbound_for_quit(app).await;

        if let (Some(store), Some(active)) = (
            app.try_state::<std::sync::Arc<MeetingStore>>(),
            app.try_state::<ActiveMeetingId>(),
        ) {
            let _ = end_active_meeting(app, store.inner(), active.inner());
        }

        if let Some(cancel) = self.watchdog_cancel.take() {
            cancel.cancel();
        }
        self.listener_shutdown.cancel();
        self.transcript_db_writer = None;
        self.transcript_tx = None;
    }

    pub(super) async fn stop_outbound_for_quit(&mut self, app: &AppHandle) {
        self.reset_mic_mute();
        self.outbound_side.starting = false;
        if self.status.0 != PipelineState::Stopping {
            self.status.0 = PipelineState::Stopping;
            self.publish_state(app);
        }
        self.flush_live_transcript_segments(app, &["outbound"]);
        self.cancel_all_outbound_tasks();
        self.outbound.stop().await;
        self.outbound_side.active_since = None;
        self.clear_bridge_state(Direction::Outbound);
        self.stop_direct_outbound_internal().await;
        self.status.0 = PipelineState::Off;
        self.last_error = None;
    }

    pub(super) async fn stop_inbound_for_quit(&mut self, app: &AppHandle) {
        self.reset_speaker_mute();
        self.inbound_side.starting = false;
        if self.status.1 != PipelineState::Stopping {
            self.status.1 = PipelineState::Stopping;
            self.publish_state(app);
        }
        self.flush_live_transcript_segments(app, &["inbound"]);
        self.cancel_all_inbound_tasks();
        self.inbound.stop().await;
        self.inbound_side.active_since = None;
        self.clear_bridge_state(Direction::Inbound);
        self.stop_direct_inbound_internal().await;
        self.status.1 = PipelineState::Off;
        self.last_error = None;
        self.publish_state(app);
    }
}
