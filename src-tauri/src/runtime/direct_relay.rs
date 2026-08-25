use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::audio::{
    resolve_role_device, AudioDeviceInfo, AudioFaultSender, AudioRole, CaptureHandle,
    CaptureHeartbeat,
};
use crate::config::AppConfig;

pub struct DirectRelay {
    capture: Option<CaptureHandle>,
    cancel: Option<CancellationToken>,
    capture_device_id: Option<String>,
    heartbeat: CaptureHeartbeat,
    started_at_ms: Option<u64>,
}

impl Default for DirectRelay {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectRelay {
    pub fn new() -> Self {
        Self {
            capture: None,
            cancel: None,
            capture_device_id: None,
            heartbeat: CaptureHeartbeat::new(),
            started_at_ms: None,
        }
    }

    pub fn started_at_ms(&self) -> Option<u64> {
        self.started_at_ms
    }

    pub fn is_active(&self) -> bool {
        self.capture.is_some()
    }

    pub fn capture_device_id(&self) -> Option<&str> {
        self.capture_device_id.as_deref()
    }

    pub fn heartbeat(&self) -> &CaptureHeartbeat {
        &self.heartbeat
    }

    pub fn is_outbound_healthy(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> bool {
        if !self.is_active() {
            return false;
        }
        let Ok(resolved) = resolve_role_device(AudioRole::UserMic, config, devices) else {
            return false;
        };
        if self.capture_device_id.as_deref() != Some(resolved.id.as_str()) {
            return false;
        }
        resolve_role_device(AudioRole::TeamsMicFeed, config, devices).is_ok()
    }

    pub fn is_inbound_healthy(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> bool {
        if !self.is_active() {
            return false;
        }
        let Ok(resolved) = resolve_role_device(AudioRole::MeetingCapture, config, devices) else {
            return false;
        };
        if self.capture_device_id.as_deref() != Some(resolved.id.as_str()) {
            return false;
        }
        resolve_role_device(AudioRole::LocalPlayback, config, devices).is_ok()
    }

    pub fn start_outbound(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        cancel: CancellationToken,
        mic_muted: Arc<AtomicBool>,
        fault_tx: Option<AudioFaultSender>,
    ) -> Result<()> {
        let user_mic = resolve_role_device(AudioRole::UserMic, config, devices)
            .map_err(|e| anyhow::anyhow!(e))?;
        let teams_mic_feed = resolve_role_device(AudioRole::TeamsMicFeed, config, devices)
            .map_err(|e| anyhow::anyhow!(e))?;
        tracing::info!("direct outbound mic → {} ({})", user_mic.name, user_mic.id);
        tracing::info!(
            "direct outbound playback → {} ({})",
            teams_mic_feed.name,
            teams_mic_feed.id
        );

        let heartbeat = CaptureHeartbeat::new();
        let capture = start_direct_relay(
            config,
            devices,
            DirectCaptureSource::UserMic,
            user_mic,
            teams_mic_feed,
            cancel.clone(),
            mic_muted,
            heartbeat.clone(),
            fault_tx,
            "direct-outbound",
        )?;

        info!("direct outbound relay started");
        self.capture_device_id = Some(capture.device_id().to_string());
        self.heartbeat = heartbeat;
        self.started_at_ms = Some(unix_ms_now());
        self.capture = Some(capture);
        self.cancel = Some(cancel);
        Ok(())
    }

    pub fn start_inbound(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        cancel: CancellationToken,
        speaker_muted: Arc<AtomicBool>,
        fault_tx: Option<AudioFaultSender>,
    ) -> Result<()> {
        let meeting_capture = resolve_role_device(AudioRole::MeetingCapture, config, devices)
            .map_err(|e| anyhow::anyhow!(e))?;
        let local_playback = resolve_role_device(AudioRole::LocalPlayback, config, devices)
            .map_err(|e| anyhow::anyhow!(e))?;
        tracing::info!(
            "direct inbound capture → {} ({})",
            meeting_capture.name,
            meeting_capture.id
        );
        tracing::info!(
            "direct inbound playback → {} ({})",
            local_playback.name,
            local_playback.id
        );

        let heartbeat = CaptureHeartbeat::new();
        let capture = start_direct_relay(
            config,
            devices,
            DirectCaptureSource::MeetingCapture,
            meeting_capture,
            local_playback,
            cancel.clone(),
            speaker_muted,
            heartbeat.clone(),
            fault_tx,
            "direct-inbound",
        )?;

        info!("direct inbound relay started");
        self.capture_device_id = Some(capture.device_id().to_string());
        self.heartbeat = heartbeat;
        self.started_at_ms = Some(unix_ms_now());
        self.capture = Some(capture);
        self.cancel = Some(cancel);
        Ok(())
    }

    /// Cancel + clear metadata under a lock; return the capture handle so the
    /// caller can `join_capture_handle` **after** dropping the engine mutex.
    pub fn take_capture_for_stop(&mut self) -> Option<CaptureHandle> {
        if let Some(cancel) = self.cancel.take() {
            cancel.cancel();
        }
        self.capture_device_id = None;
        self.heartbeat = CaptureHeartbeat::new();
        self.started_at_ms = None;
        self.capture.take()
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.take_capture_for_stop() {
            join_capture_handle(handle).await;
        }
    }
}

/// Join a capture thread off the engine mutex (and off the tokio worker).
pub async fn join_capture_handle(handle: CaptureHandle) {
    let _ = tokio::task::spawn_blocking(move || handle.stop()).await;
}

enum DirectCaptureSource {
    UserMic,
    MeetingCapture,
}

fn unix_ms_now() -> u64 {
    crate::audio::monotonic_ms()
}

#[cfg(any(windows, target_os = "macos"))]
fn start_direct_relay(
    _config: &AppConfig,
    _devices: &[AudioDeviceInfo],
    _capture_source: DirectCaptureSource,
    capture_device: crate::audio::ResolvedDevice,
    playback_device: crate::audio::ResolvedDevice,
    _cancel: CancellationToken,
    mute_gate: Arc<AtomicBool>,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<AudioFaultSender>,
    thread_name: &'static str,
) -> Result<CaptureHandle> {
    crate::audio::start_direct_passthrough(
        capture_device,
        playback_device,
        mute_gate,
        heartbeat,
        fault_tx,
        thread_name,
    )
}

#[cfg(not(any(windows, target_os = "macos")))]
fn start_direct_relay(
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
    capture_source: DirectCaptureSource,
    _capture_device: crate::audio::ResolvedDevice,
    playback_device: crate::audio::ResolvedDevice,
    cancel: CancellationToken,
    mute_gate: Arc<AtomicBool>,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<AudioFaultSender>,
    _thread_name: &'static str,
) -> Result<CaptureHandle> {
    use tokio::sync::mpsc;

    use crate::audio::{
        gate_pcm_in_place, send_passthrough, shared_playback_device, spawn_pcm24k_adapter,
        spawn_pipeline_audio, start_meeting_capture_for_config, start_user_mic_capture,
        AudioModeHandle, CAPTURE_CHANNEL_DEPTH,
    };
    use crate::config::PipelineOutputMode;

    const PASSTHROUGH_DEPTH: usize = 6;

    let audio_mode = AudioModeHandle::new(PipelineOutputMode::OriginalAudio);
    let (_gemini_tx, bridge_audio_rx) =
        mpsc::channel::<Vec<i16>>(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
    let (passthrough_tx, passthrough_rx) = mpsc::channel(PASSTHROUGH_DEPTH);
    let (capture_tx, capture_rx) = mpsc::channel(CAPTURE_CHANNEL_DEPTH);

    let capture = match capture_source {
        DirectCaptureSource::UserMic => {
            start_user_mic_capture(config, devices, capture_tx, heartbeat.clone(), fault_tx)?
        }
        DirectCaptureSource::MeetingCapture => start_meeting_capture_for_config(
            config,
            devices,
            capture_tx,
            heartbeat.clone(),
            fault_tx,
        )?,
    };

    let pcm_drops = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let playback_chunks = spawn_pcm24k_adapter(cancel.clone(), bridge_audio_rx, pcm_drops.clone());
    let playback_shared = shared_playback_device(playback_device);

    spawn_pipeline_audio(
        cancel.clone(),
        audio_mode.shared_mode(),
        audio_mode.shared_generation(),
        playback_shared,
        playback_chunks,
        passthrough_rx,
        None,
        None,
        None,
        None,
        None,
        pcm_drops,
    );

    let passthrough_tx_bg = passthrough_tx.clone();
    tokio::spawn(async move {
        let mut capture_rx = capture_rx;
        while !cancel.is_cancelled() {
            match capture_rx.recv().await {
                Some(mut pcm) => {
                    gate_pcm_in_place(&mute_gate, &mut pcm);
                    send_passthrough(&passthrough_tx_bg, pcm);
                }
                None => break,
            }
        }
    });

    Ok(capture)
}
