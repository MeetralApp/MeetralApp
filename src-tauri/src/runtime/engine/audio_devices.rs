use super::types::PipelineState;
use super::{SharedEngine, TranslationEngine};
use crate::audio::AudioDeviceInfo;
use crate::config::{AppConfig, DeviceRef};
use tauri::AppHandle;

fn device_id_changed(old: &DeviceRef, new: &DeviceRef) -> bool {
    old.id.trim() != new.id.trim()
}

fn outbound_devices_changed(old: &AppConfig, new: &AppConfig) -> bool {
    device_id_changed(&old.user_mic, &new.user_mic)
        || device_id_changed(&old.teams_mic_feed, &new.teams_mic_feed)
}

fn inbound_devices_changed(old: &AppConfig, new: &AppConfig) -> bool {
    device_id_changed(&old.meeting_capture, &new.meeting_capture)
        || device_id_changed(&old.local_playback, &new.local_playback)
}

/// Result of hot-rewiring WASAPI capture/playback after a config save.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevicesApplyResult {
    /// True when at least one live audio path was restarted/rebound.
    pub rewired: bool,
}

impl TranslationEngine {
    /// Hot-apply audio device roles after config save while pipelines stay up.
    ///
    /// Keeps STT/TTS bridges; only rewires capture/playback (and direct relays).
    /// Restarts only directions whose device IDs actually changed.
    pub async fn apply_audio_devices_after_config_change(
        &mut self,
        old: &AppConfig,
        new: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
        engine: SharedEngine,
    ) -> Result<AudioDevicesApplyResult, String> {
        let outbound_changed = outbound_devices_changed(old, new);
        let inbound_changed = inbound_devices_changed(old, new);
        if !outbound_changed && !inbound_changed {
            return Ok(AudioDevicesApplyResult { rewired: false });
        }

        let mut rewired = false;

        if outbound_changed {
            rewired |= self.apply_outbound_audio_devices(new, devices, app).await?;
        }
        if inbound_changed {
            rewired |= self.apply_inbound_audio_devices(new, devices, app).await?;
        }

        if rewired {
            self.ensure_watchdog(app.clone(), engine);
            self.publish_state(app);
        }

        Ok(AudioDevicesApplyResult { rewired })
    }

    async fn apply_outbound_audio_devices(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<bool, String> {
        if self.status.0 == PipelineState::Active && self.outbound.is_active() {
            let previous = self
                .outbound
                .restart_audio_path(config, devices, self.outbound_audio_fault_tx())
                .map_err(|e| format!("Failed to apply outbound audio devices: {e:#}"))?;
            super::lock_scope::join_optional_capture(previous).await;
            tracing::info!("outbound audio devices hot-applied after config save");
            return Ok(true);
        }

        if self.is_outbound_busy() {
            return Ok(false);
        }

        self.restart_direct_outbound_for_device_change(config, devices, app)
            .await
    }

    async fn apply_inbound_audio_devices(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<bool, String> {
        if self.status.1 == PipelineState::Active && self.inbound.is_active() {
            let previous = self
                .inbound
                .restart_audio_path(config, devices, self.inbound_audio_fault_tx())
                .map_err(|e| format!("Failed to apply inbound audio devices: {e:#}"))?;
            super::lock_scope::join_optional_capture(previous).await;
            tracing::info!("inbound audio devices hot-applied after config save");
            return Ok(true);
        }

        if self.is_inbound_busy() {
            return Ok(false);
        }

        self.restart_direct_inbound_for_device_change(config, devices, app)
            .await
    }

    async fn restart_direct_outbound_for_device_change(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<bool, String> {
        if !config.keep_direct_audio {
            return Ok(false);
        }

        let was_direct =
            self.outbound_side.direct.is_active() || self.status.0 == PipelineState::Direct;
        if was_direct {
            self.stop_direct_outbound_internal().await;
            self.status.0 = PipelineState::Off;
        }

        if config.validate_for_direct_outbound(devices).is_ok() {
            self.start_direct_outbound(config, devices, app)?;
            tracing::info!("direct outbound restarted after audio device change");
            return Ok(true);
        }

        if was_direct {
            self.publish_state(app);
            return Ok(true);
        }
        Ok(false)
    }

    async fn restart_direct_inbound_for_device_change(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<bool, String> {
        if !config.keep_direct_audio {
            return Ok(false);
        }

        let was_direct =
            self.inbound_side.direct.is_active() || self.status.1 == PipelineState::Direct;
        if was_direct {
            self.stop_direct_inbound_internal().await;
            self.status.1 = PipelineState::Off;
        }

        if config.validate_for_direct_inbound(devices).is_ok() {
            self.start_direct_inbound(config, devices, app)?;
            tracing::info!("direct inbound restarted after audio device change");
            return Ok(true);
        }

        if was_direct {
            self.publish_state(app);
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{inbound_devices_changed, outbound_devices_changed};
    use crate::config::{AppConfig, DeviceRef};

    fn with_devices(user_mic: &str, teams: &str, capture: &str, playback: &str) -> AppConfig {
        AppConfig {
            user_mic: DeviceRef {
                id: user_mic.into(),
                name: user_mic.into(),
            },
            teams_mic_feed: DeviceRef {
                id: teams.into(),
                name: teams.into(),
            },
            meeting_capture: DeviceRef {
                id: capture.into(),
                name: capture.into(),
            },
            local_playback: DeviceRef {
                id: playback.into(),
                name: playback.into(),
            },
            ..AppConfig::default()
        }
    }

    #[test]
    fn detects_outbound_device_id_changes_only() {
        let a = with_devices("mic1", "teams1", "cap1", "hp1");
        let b = with_devices("mic2", "teams1", "cap1", "hp1");
        assert!(outbound_devices_changed(&a, &b));
        assert!(!inbound_devices_changed(&a, &b));
    }

    #[test]
    fn detects_inbound_playback_change() {
        let a = with_devices("mic1", "teams1", "cap1", "hp1");
        let b = with_devices("mic1", "teams1", "cap1", "hp2");
        assert!(!outbound_devices_changed(&a, &b));
        assert!(inbound_devices_changed(&a, &b));
    }
}
