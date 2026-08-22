use super::{
    Direction, PipelineState, TranslationEngine, AUDIO_CATALOG_STABLE_MS, AUDIO_HEARTBEAT_STALE_MS,
    AUDIO_STARTUP_GRACE_MS,
};
use crate::audio::{AudioDeviceInfo, AudioFaultSender};
use crate::config::AppConfig;

impl TranslationEngine {
    pub(super) fn note_catalog_change(&mut self, now_ms: u64) {
        self.last_catalog_change_ms = Some(now_ms);
    }

    /// Earliest time the device catalog is considered settled.
    pub(super) fn catalog_stable_at(&self) -> Option<u64> {
        self.last_catalog_change_ms
            .map(|changed_ms| changed_ms + AUDIO_CATALOG_STABLE_MS)
    }

    pub(super) fn catalog_stable(&self, now_ms: u64) -> bool {
        self.catalog_stable_at()
            .is_none_or(|stable_at| now_ms >= stable_at)
    }

    pub(super) fn try_recover_lost_audio_paths(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
    ) {
        if self.outbound_side.audio_path.is_lost()
            && !self.is_outbound_busy()
            && config.validate_for_direct_outbound(devices).is_ok()
        {
            self.outbound_side.audio_path.clear_to_ok();
        }
        if self.inbound_side.audio_path.is_lost()
            && !self.is_inbound_busy()
            && config.validate_for_direct_inbound(devices).is_ok()
        {
            self.inbound_side.audio_path.clear_to_ok();
        }
    }

    pub(super) fn clear_audio_state(&mut self, direction: Direction) {
        self.side_mut(direction).audio_path.clear_to_ok();
    }

    pub(super) fn clear_outbound_audio_state(&mut self) {
        self.clear_audio_state(Direction::Outbound);
    }

    pub(super) fn clear_inbound_audio_state(&mut self) {
        self.clear_audio_state(Direction::Inbound);
    }

    pub(super) fn capture_heartbeat_confirmed(
        heartbeat: &crate::audio::CaptureHeartbeat,
        started_at_ms: Option<u64>,
        now_ms: u64,
    ) -> bool {
        if heartbeat.last_frame_ms() == 0 {
            return false;
        }
        !heartbeat.is_stale(
            now_ms,
            AUDIO_HEARTBEAT_STALE_MS,
            started_at_ms,
            AUDIO_STARTUP_GRACE_MS,
        )
    }

    fn pipeline_status(&self, direction: Direction) -> &PipelineState {
        match direction {
            Direction::Outbound => &self.status.0,
            Direction::Inbound => &self.status.1,
        }
    }

    fn direct_healthy(
        &self,
        direction: Direction,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
    ) -> bool {
        let direct = &self.side(direction).direct;
        match direction {
            Direction::Outbound => direct.is_outbound_healthy(config, devices),
            Direction::Inbound => direct.is_inbound_healthy(config, devices),
        }
    }

    fn translate_active(&self, direction: Direction) -> bool {
        match direction {
            Direction::Outbound => self.outbound.is_active(),
            Direction::Inbound => self.inbound.is_active(),
        }
    }

    fn translate_path_healthy(
        &self,
        direction: Direction,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
    ) -> bool {
        match direction {
            Direction::Outbound => self.outbound.is_audio_path_healthy(config, devices),
            Direction::Inbound => self.inbound.is_audio_path_healthy(config, devices),
        }
    }

    fn translate_heartbeat(
        &self,
        direction: Direction,
    ) -> (&crate::audio::CaptureHeartbeat, Option<u64>) {
        match direction {
            Direction::Outbound => (self.outbound.heartbeat(), self.outbound.started_at_ms()),
            Direction::Inbound => (self.inbound.heartbeat(), self.inbound.started_at_ms()),
        }
    }

    pub(super) fn audio_recovery_confirmed(
        &self,
        direction: Direction,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        now_ms: u64,
    ) -> bool {
        let side = self.side(direction);
        match self.pipeline_status(direction) {
            PipelineState::Direct if side.direct.is_active() => {
                self.direct_healthy(direction, config, devices)
                    && Self::capture_heartbeat_confirmed(
                        side.direct.heartbeat(),
                        side.direct.started_at_ms(),
                        now_ms,
                    )
            }
            PipelineState::Active if self.translate_active(direction) => {
                let (heartbeat, started) = self.translate_heartbeat(direction);
                self.translate_path_healthy(direction, config, devices)
                    && Self::capture_heartbeat_confirmed(heartbeat, started, now_ms)
            }
            _ => false,
        }
    }

    pub(super) fn audio_still_unhealthy(
        &self,
        direction: Direction,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        now_ms: u64,
    ) -> bool {
        let side = self.side(direction);
        match self.pipeline_status(direction) {
            PipelineState::Direct if side.direct.is_active() => {
                !self.direct_healthy(direction, config, devices)
                    || side.direct.heartbeat().is_stale(
                        now_ms,
                        AUDIO_HEARTBEAT_STALE_MS,
                        side.direct.started_at_ms(),
                        AUDIO_STARTUP_GRACE_MS,
                    )
            }
            PipelineState::Direct | PipelineState::Off => true,
            PipelineState::Active if self.translate_active(direction) => {
                let (heartbeat, started) = self.translate_heartbeat(direction);
                !self.translate_path_healthy(direction, config, devices)
                    || heartbeat.is_stale(
                        now_ms,
                        AUDIO_HEARTBEAT_STALE_MS,
                        started,
                        AUDIO_STARTUP_GRACE_MS,
                    )
            }
            _ => true,
        }
    }

    pub(super) fn audio_fault_tx(&self, direction: Direction) -> Option<AudioFaultSender> {
        self.side(direction).audio_fault_tx.clone()
    }

    pub(super) fn outbound_audio_fault_tx(&self) -> Option<AudioFaultSender> {
        self.audio_fault_tx(Direction::Outbound)
    }

    pub(super) fn inbound_audio_fault_tx(&self) -> Option<AudioFaultSender> {
        self.audio_fault_tx(Direction::Inbound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_stable_when_never_changed() {
        let engine = TranslationEngine::new();
        assert!(engine.catalog_stable(1_000));
        assert_eq!(engine.catalog_stable_at(), None);
    }

    #[test]
    fn catalog_stability_window_after_change() {
        let mut engine = TranslationEngine::new();
        engine.note_catalog_change(1_000);
        assert_eq!(
            engine.catalog_stable_at(),
            Some(1_000 + AUDIO_CATALOG_STABLE_MS)
        );
        assert!(!engine.catalog_stable(1_000));
        assert!(!engine.catalog_stable(1_000 + AUDIO_CATALOG_STABLE_MS - 1));
        assert!(engine.catalog_stable(1_000 + AUDIO_CATALOG_STABLE_MS));
    }

    #[test]
    fn later_change_extends_stability_window() {
        let mut engine = TranslationEngine::new();
        engine.note_catalog_change(1_000);
        let second_change = 1_000 + AUDIO_CATALOG_STABLE_MS - 1;
        engine.note_catalog_change(second_change);
        // Past the first change's settle point, but the window moved.
        assert!(!engine.catalog_stable(1_000 + AUDIO_CATALOG_STABLE_MS));
        assert!(!engine.catalog_stable(second_change + AUDIO_CATALOG_STABLE_MS - 1));
        assert!(engine.catalog_stable(second_change + AUDIO_CATALOG_STABLE_MS));
    }
}
