use anyhow::Result;
use tokio::sync::mpsc;

use crate::audio::{
    resolve_role_device, start_user_mic_capture, AudioDeviceInfo, AudioFaultSender, AudioRole,
    CaptureHeartbeat, ResolvedDevice, CAPTURE_CHANNEL_DEPTH,
};
use crate::config::AppConfig;

use super::{unix_ms_now, OutboundPipeline};

impl OutboundPipeline {
    /// Stop the current capture without joining; caller joins outside the engine lock.
    pub fn take_capture_for_stop(&mut self) -> Option<crate::audio::CaptureHandle> {
        self.capture.take()
    }

    /// Restart capture; returns the previous handle to join outside any engine lock.
    pub fn restart_capture(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        audio_fault_tx: Option<AudioFaultSender>,
    ) -> Result<Option<crate::audio::CaptureHandle>> {
        let previous = self.capture.take();

        let attach_tx = self
            .capture_attach_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("capture forward not running"))?;

        let (capture_tx, capture_rx) = mpsc::channel(CAPTURE_CHANNEL_DEPTH);
        let heartbeat = CaptureHeartbeat::new();
        let capture = start_user_mic_capture(
            config,
            devices,
            capture_tx,
            heartbeat.clone(),
            audio_fault_tx,
        )?;

        attach_tx
            .try_send(capture_rx)
            .map_err(|e| anyhow::anyhow!("failed to attach capture forward: {e}"))?;

        self.capture_device_id = Some(capture.device_id().to_string());
        self.heartbeat = heartbeat;
        self.started_at_ms = Some(unix_ms_now());
        self.capture = Some(capture);
        Ok(previous)
    }

    pub fn restart_playback(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> Result<()> {
        let handle = self
            .audio_mode
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("audio mode not running"))?;
        let shared = self
            .playback_device
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("playback device not running"))?;

        let next = if config.outbound_mode.needs_playback() {
            resolve_role_device(AudioRole::TeamsMicFeed, config, devices)
                .map_err(|e| anyhow::anyhow!(e))?
        } else {
            ResolvedDevice {
                id: String::new(),
                name: String::new(),
                direction: "output",
            }
        };

        {
            let mut guard = shared
                .lock()
                .map_err(|_| anyhow::anyhow!("playback device lock poisoned"))?;
            if guard.id != next.id {
                tracing::info!("outbound playback hot-swap → {} ({})", next.name, next.id);
            }
            *guard = next;
        }

        handle
            .set_mode(handle.current_mode())
            .map_err(|e| anyhow::anyhow!(e))?;
        if let Some(runtime) = &self.voice_runtime {
            runtime.bump_mux_generation();
        }
        Ok(())
    }

    pub fn restart_audio_path(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        audio_fault_tx: Option<AudioFaultSender>,
    ) -> Result<Option<crate::audio::CaptureHandle>> {
        let previous = self.restart_capture(config, devices, audio_fault_tx)?;
        self.restart_playback(config, devices)?;
        Ok(previous)
    }
}
