use super::types::PipelineState;
use super::TranslationEngine;
use crate::audio::AudioDeviceInfo;
use crate::config::AppConfig;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

impl TranslationEngine {
    pub fn start_direct_outbound(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<(), String> {
        if self.is_outbound_busy() {
            return Err("Outbound translation is active".into());
        }

        if self.status.0 == PipelineState::Direct
            && self
                .outbound_side
                .direct
                .is_outbound_healthy(config, devices)
        {
            if !self.outbound_side.audio_path.is_ok() {
                self.clear_outbound_audio_state();
                self.publish_state(app);
            }
            return Ok(());
        }
        if self
            .outbound_side
            .direct
            .is_outbound_healthy(config, devices)
            && !self.is_outbound_busy()
        {
            if self.status.0 != PipelineState::Direct {
                self.status.0 = PipelineState::Direct;
                self.last_error = None;
                self.publish_state(app);
            }
            if !self.outbound_side.audio_path.is_ok() {
                self.clear_outbound_audio_state();
                self.publish_state(app);
            }
            return Ok(());
        }
        if self.outbound_side.direct.is_active() {
            return Err("Direct outbound relay is already active".into());
        }

        config
            .validate_for_direct_outbound(devices)
            .map_err(|e| e.to_string())?;

        let cancel = CancellationToken::new();
        let mic_muted = self.mic_mute.shared();
        self.outbound_side
            .direct
            .start_outbound(
                config,
                devices,
                cancel,
                mic_muted,
                self.outbound_audio_fault_tx(),
            )
            .map_err(|e| e.to_string())?;
        self.status.0 = PipelineState::Direct;
        self.last_error = None;
        self.clear_outbound_audio_state();
        self.publish_state(app);
        Ok(())
    }

    pub fn start_direct_inbound(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<(), String> {
        if self.is_inbound_busy() {
            return Err("Inbound translation is active".into());
        }

        if self.status.1 == PipelineState::Direct
            && self.inbound_side.direct.is_inbound_healthy(config, devices)
        {
            if !self.inbound_side.audio_path.is_ok() {
                self.clear_inbound_audio_state();
                self.publish_state(app);
            }
            return Ok(());
        }
        if self.inbound_side.direct.is_inbound_healthy(config, devices) && !self.is_inbound_busy() {
            if self.status.1 != PipelineState::Direct {
                self.status.1 = PipelineState::Direct;
                self.last_error = None;
                self.publish_state(app);
            }
            if !self.inbound_side.audio_path.is_ok() {
                self.clear_inbound_audio_state();
                self.publish_state(app);
            }
            return Ok(());
        }
        if self.inbound_side.direct.is_active() {
            return Err("Direct inbound relay is already active".into());
        }

        config
            .validate_for_direct_inbound(devices)
            .map_err(|e| e.to_string())?;

        let cancel = CancellationToken::new();
        let speaker_muted = self.speaker_mute.shared();
        self.inbound_side
            .direct
            .start_inbound(
                config,
                devices,
                cancel,
                speaker_muted,
                self.inbound_audio_fault_tx(),
            )
            .map_err(|e| e.to_string())?;
        self.status.1 = PipelineState::Direct;
        self.last_error = None;
        self.clear_inbound_audio_state();
        self.publish_state(app);
        Ok(())
    }

    pub async fn stop_direct_outbound(&mut self, app: &AppHandle) {
        self.stop_direct_outbound_internal().await;
        if self.status.0 == PipelineState::Direct {
            self.status.0 = PipelineState::Off;
            self.publish_state(app);
        }
    }

    pub async fn stop_direct_inbound(&mut self, app: &AppHandle) {
        self.stop_direct_inbound_internal().await;
        if self.status.1 == PipelineState::Direct {
            self.status.1 = PipelineState::Off;
            self.publish_state(app);
        }
    }

    /// Synchronous: safe to call under the engine lock (no `.await` inside).
    pub fn resume_direct_outbound_if_enabled(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) {
        if config.keep_direct_audio {
            if let Err(e) = self.start_direct_outbound(config, devices, app) {
                tracing::warn!("failed to resume direct outbound: {e}");
                self.status.0 = PipelineState::Off;
                self.publish_state(app);
            }
        } else {
            self.status.0 = PipelineState::Off;
            self.publish_state(app);
        }
    }

    /// Synchronous: safe to call under the engine lock (no `.await` inside).
    pub fn resume_direct_inbound_if_enabled(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) {
        if config.keep_direct_audio {
            if let Err(e) = self.start_direct_inbound(config, devices, app) {
                tracing::warn!("failed to resume direct inbound: {e}");
                self.status.1 = PipelineState::Off;
                self.publish_state(app);
            }
        } else {
            self.status.1 = PipelineState::Off;
            self.publish_state(app);
        }
    }

    /// Take standby direct relays for stop without joining under the caller's
    /// engine guard; join the returned handles after dropping the guard.
    pub(crate) fn take_standby_direct_audio_for_stop(
        &mut self,
        app: &AppHandle,
    ) -> Vec<crate::audio::CaptureHandle> {
        let mut handles = Vec::new();
        if !self.is_outbound_busy()
            && (self.outbound_side.direct.is_active() || self.status.0 == PipelineState::Direct)
        {
            if let Some(handle) = self.take_direct_outbound_for_stop() {
                handles.push(handle);
            }
            if self.status.0 == PipelineState::Direct {
                self.status.0 = PipelineState::Off;
            }
        }
        if !self.is_inbound_busy()
            && (self.inbound_side.direct.is_active() || self.status.1 == PipelineState::Direct)
        {
            if let Some(handle) = self.take_direct_inbound_for_stop() {
                handles.push(handle);
            }
            if self.status.1 == PipelineState::Direct {
                self.status.1 = PipelineState::Off;
            }
        }
        self.publish_state(app);
        handles
    }
}
