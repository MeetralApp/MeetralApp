mod audio_path;
mod bridge;
mod lifecycle;
mod session;

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::audio::{
    resolve_role_device, AudioDeviceInfo, AudioModeHandle, AudioRole, CaptureHandle,
    CaptureHeartbeat, SharedPlaybackDevice,
};
use crate::config::{AppConfig, PipelineOutputMode};
use crate::providers::shared::live::LiveBridgeHandle;
use crate::runtime::voice_runtime::OutboundVoiceRuntime;
use crate::voice::{types::VoiceCustomLatencyEvent, VoiceTtsStatus};

pub(crate) use lifecycle::{
    await_outbound_session_tasks, teardown_outbound_parts, OutboundTeardown,
};

pub(crate) const PASSTHROUGH_DEPTH: usize = 6;
pub(crate) const SESSION_TASK_JOIN_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(100);

pub(crate) struct OutboundSessionTasks {
    pub(crate) tts_worker: Option<tokio::task::JoinHandle<()>>,
    pub(crate) fanout: tokio::task::JoinHandle<()>,
    pub(crate) mux: Option<tokio::task::JoinHandle<()>>,
    pub(crate) idle_watcher: Option<tokio::task::JoinHandle<()>>,
}

pub(crate) struct OutboundStartConnect {
    pub(crate) bridge: LiveBridgeHandle,
    pub(crate) playback_rx: mpsc::Receiver<crate::audio::PlaybackPcmChunk>,
    pub(crate) audio_mode: Option<AudioModeHandle>,
    pub(crate) session_tasks: Option<OutboundSessionTasks>,
    pub(crate) voice_runtime: Option<Arc<OutboundVoiceRuntime>>,
    pub(crate) voice_tts_status_rx: Option<mpsc::Receiver<VoiceTtsStatus>>,
    pub(crate) voice_latency_rx: Option<mpsc::Receiver<VoiceCustomLatencyEvent>>,
}

pub struct OutboundPipeline {
    capture: Option<CaptureHandle>,
    bridge: Option<LiveBridgeHandle>,
    audio_mode: Option<AudioModeHandle>,
    session_tasks: Option<OutboundSessionTasks>,
    voice_runtime: Option<Arc<OutboundVoiceRuntime>>,
    heartbeat: CaptureHeartbeat,
    capture_device_id: Option<String>,
    playback_device: Option<SharedPlaybackDevice>,
    started_at_ms: Option<u64>,
    capture_attach_tx: Option<mpsc::Sender<mpsc::Receiver<Vec<i16>>>>,
}

impl Default for OutboundPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl OutboundPipeline {
    pub fn new() -> Self {
        Self {
            capture: None,
            bridge: None,
            audio_mode: None,
            session_tasks: None,
            voice_runtime: None,
            heartbeat: CaptureHeartbeat::new(),
            capture_device_id: None,
            playback_device: None,
            started_at_ms: None,
            capture_attach_tx: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.bridge.is_some()
    }

    pub fn voice_runtime(&self) -> Option<Arc<OutboundVoiceRuntime>> {
        self.voice_runtime.clone()
    }

    pub fn heartbeat(&self) -> &CaptureHeartbeat {
        &self.heartbeat
    }

    pub fn capture_device_id(&self) -> Option<&str> {
        self.capture_device_id.as_deref()
    }

    pub fn started_at_ms(&self) -> Option<u64> {
        self.started_at_ms
    }

    pub fn is_capture_healthy(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> bool {
        if self.capture.is_none() {
            return false;
        }
        let Ok(resolved) = resolve_role_device(AudioRole::UserMic, config, devices) else {
            return false;
        };
        self.capture_device_id.as_deref() == Some(resolved.id.as_str())
    }

    pub fn is_playback_healthy(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> bool {
        if !config.outbound_mode.needs_playback() {
            return true;
        }
        let Ok(resolved) = resolve_role_device(AudioRole::TeamsMicFeed, config, devices) else {
            return false;
        };
        let Some(shared) = self.playback_device.as_ref() else {
            return false;
        };
        shared.lock().map(|d| d.id == resolved.id).unwrap_or(false)
    }

    pub fn is_audio_path_healthy(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> bool {
        self.is_capture_healthy(config, devices) && self.is_playback_healthy(config, devices)
    }

    pub fn set_audio_mode(&self, mode: PipelineOutputMode) -> Result<(), String> {
        let handle = self
            .audio_mode
            .as_ref()
            .ok_or_else(|| "Outbound pipeline is not running".to_string())?;
        handle.set_mode(mode)?;
        if let Some(runtime) = &self.voice_runtime {
            crate::runtime::voice_runtime::sync_bridge_play_audio(runtime);
            runtime.bump_mux_generation();
        }
        Ok(())
    }

    pub fn flush_playback_audio(&self) -> Result<(), String> {
        if let Some(handle) = self.audio_mode.as_ref() {
            handle.set_mode(handle.current_mode())?;
        }
        if let Some(runtime) = &self.voice_runtime {
            runtime.bump_mux_generation();
        }
        Ok(())
    }
}

pub(crate) fn unix_ms_now() -> u64 {
    crate::audio::monotonic_ms()
}
