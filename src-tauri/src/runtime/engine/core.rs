use super::types::PipelineState;
use super::{Direction, TranslationEngine};
use tauri::AppHandle;

impl TranslationEngine {
    pub(crate) fn reset_mic_mute(&mut self) {
        self.mic_mute.set_muted(false);
    }

    pub(super) fn reset_speaker_mute(&mut self) {
        self.speaker_mute.set_muted(false);
    }

    pub(super) fn is_audio_active(&self, direction: Direction) -> bool {
        matches!(
            match direction {
                Direction::Outbound => &self.status.0,
                Direction::Inbound => &self.status.1,
            },
            PipelineState::Direct
                | PipelineState::Starting
                | PipelineState::Stopping
                | PipelineState::Active
        )
    }

    pub(super) fn is_outbound_audio_active(&self) -> bool {
        self.is_audio_active(Direction::Outbound)
    }

    pub(super) fn is_inbound_audio_active(&self) -> bool {
        self.is_audio_active(Direction::Inbound)
    }

    pub(super) fn is_direction_busy(&self, direction: Direction) -> bool {
        // Intentionally omit PipelineState::Stopping: stop_* clears cancel tokens
        // then calls resume_direct_*, which must not see "translation still active".
        let side = self.side(direction);
        side.cancel.is_some() || side.starting
    }

    pub(crate) fn is_outbound_busy(&self) -> bool {
        self.is_direction_busy(Direction::Outbound)
    }

    pub(crate) fn is_inbound_busy(&self) -> bool {
        self.is_direction_busy(Direction::Inbound)
    }

    pub(super) fn is_busy(&self) -> bool {
        self.is_outbound_busy() || self.is_inbound_busy()
    }

    /// Cancel every async task token for a direction (pending start + active session).
    pub(super) fn cancel_all_direction_tasks(&mut self, direction: Direction) {
        let side = self.side_mut(direction);
        if let Some(cancel) = side.pending_cancel.take() {
            cancel.cancel();
        }
        if let Some(cancel) = side.cancel.take() {
            cancel.cancel();
        }
    }

    /// Cancel every outbound async task token (pending start + active session).
    pub(super) fn cancel_all_outbound_tasks(&mut self) {
        self.cancel_all_direction_tasks(Direction::Outbound);
    }

    /// Cancel every inbound async task token (pending start + active session).
    pub(super) fn cancel_all_inbound_tasks(&mut self) {
        self.cancel_all_direction_tasks(Direction::Inbound);
    }

    /// Take the direct capture for stop (no join). Join via
    /// [`crate::runtime::direct_relay::join_capture_handle`] outside the engine lock.
    pub(crate) fn take_direct_for_stop(
        &mut self,
        direction: Direction,
    ) -> Option<crate::audio::CaptureHandle> {
        match direction {
            Direction::Outbound => self.reset_mic_mute(),
            Direction::Inbound => self.reset_speaker_mute(),
        }
        self.side_mut(direction).direct.take_capture_for_stop()
    }

    pub(crate) fn take_direct_outbound_for_stop(&mut self) -> Option<crate::audio::CaptureHandle> {
        self.take_direct_for_stop(Direction::Outbound)
    }

    pub(crate) fn take_direct_inbound_for_stop(&mut self) -> Option<crate::audio::CaptureHandle> {
        self.take_direct_for_stop(Direction::Inbound)
    }

    /// Convenience when the caller already owns exclusive engine access without
    /// holding [`super::SharedEngine`] across the join.
    pub(crate) async fn stop_direct_internal(&mut self, direction: Direction) {
        if let Some(handle) = self.take_direct_for_stop(direction) {
            crate::runtime::direct_relay::join_capture_handle(handle).await;
        }
    }

    pub(crate) async fn stop_direct_outbound_internal(&mut self) {
        self.stop_direct_internal(Direction::Outbound).await;
    }

    pub(crate) async fn stop_direct_inbound_internal(&mut self) {
        self.stop_direct_internal(Direction::Inbound).await;
    }

    pub(super) fn publish_state(&mut self, app: &AppHandle) {
        self.publish(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn cancel_all_outbound_tasks_cancels_pending_and_active() {
        let mut engine = TranslationEngine::new();
        let pending = CancellationToken::new();
        let active = CancellationToken::new();
        engine.outbound_side.pending_cancel = Some(pending.clone());
        engine.outbound_side.cancel = Some(active.clone());
        engine.cancel_all_outbound_tasks();
        assert!(pending.is_cancelled());
        assert!(active.is_cancelled());
        assert!(engine.outbound_side.pending_cancel.is_none());
        assert!(engine.outbound_side.cancel.is_none());
    }

    #[test]
    fn stopping_status_alone_is_not_busy() {
        let mut engine = TranslationEngine::new();
        engine.status.0 = super::super::types::PipelineState::Stopping;
        engine.status.1 = super::super::types::PipelineState::Stopping;
        assert!(!engine.is_outbound_busy());
        assert!(!engine.is_inbound_busy());
    }
}
