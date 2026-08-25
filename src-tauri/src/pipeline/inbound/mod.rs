mod audio_path;
mod bridge;
mod lifecycle;
mod provider_tts;
mod session;

pub(crate) use bridge::inbound_voice_output_to_engine;
pub(crate) use lifecycle::{teardown_inbound_parts, InboundTeardown};

use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8},
    Arc, Mutex as StdMutex,
};
use std::time::Instant;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::audio::ducking_mix::DuckingParams;
use crate::audio::PlaybackPcmChunk;
use crate::audio::{
    resolve_role_device, AudioDeviceInfo, AudioModeHandle, AudioRole, CaptureHandle,
    CaptureHeartbeat, SharedPlaybackDevice,
};
use crate::config::{AppConfig, PipelineOutputMode};
use crate::providers::shared::live::LiveBridgeHandle;
use crate::runtime::voice_runtime::OutboundTtsSession;
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::TtsTextCommand;

pub(crate) const PASSTHROUGH_DEPTH: usize = 6;

/// Result of inbound live-bridge connect (STT + optional Soniox TTS fanout).
pub struct InboundBridgeConnect {
    pub bridge: LiveBridgeHandle,
    /// Legacy raw PCM (Gemini/OpenAI STS, or unused when `playback_chunks` is set).
    pub playback_rx: mpsc::Receiver<Vec<i16>>,
    /// Soniox TTS path: chunked 24 kHz PCM.
    pub playback_chunks: Option<mpsc::Receiver<PlaybackPcmChunk>>,
    /// Shared with fanout when Soniox TTS is scaffolded (any inbound mode under Soniox).
    pub audio_mode: Option<AudioModeHandle>,
    /// Soniox TTS scaffold; `session` is set while Translated (hot-started from Raw/Captions).
    pub provider_tts: Option<InboundProviderTts>,
    /// Shared overflow counter for bounded PCM queues (engine `pcm_frames_dropped`).
    pub pcm_drops: Arc<AtomicU64>,
}

/// Inbound text-to-speech runtime, independent from the outbound voice workers.
pub struct InboundProviderTts {
    pub tts_cmd_tx: Arc<StdMutex<mpsc::Sender<TtsTextCommand>>>,
    pub pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    pub provider_session: Option<OutboundTtsSession>,
    pub custom_session: Option<OutboundTtsSession>,
    /// Parent of worker cancels; cancelled with the inbound pipeline.
    pub pipeline_cancel: CancellationToken,
    pub voice_engine: Arc<AtomicU8>,
    pub bridge_play_audio: Arc<AtomicBool>,
    pub mux_generation: Arc<AtomicU64>,
    pub voice_switch_in_progress: Arc<AtomicBool>,
    pub voice_switch_mutex: Arc<tokio::sync::Mutex<()>>,
    pub last_switch_at: Arc<StdMutex<Instant>>,
    pub provider_tts_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    pub custom_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    pub playback_generation: Arc<AtomicU8>,
    pub turn_latency: Arc<TurnLatencySlot>,
    pub pcm_drops: Arc<AtomicU64>,
}

pub struct InboundPipeline {
    capture: Option<CaptureHandle>,
    bridge: Option<LiveBridgeHandle>,
    audio_mode: Option<AudioModeHandle>,
    heartbeat: CaptureHeartbeat,
    capture_device_id: Option<String>,
    playback_device: Option<SharedPlaybackDevice>,
    started_at_ms: Option<u64>,
    capture_attach_tx: Option<mpsc::Sender<mpsc::Receiver<Vec<i16>>>>,
    provider_tts: Option<InboundProviderTts>,
    ducking_params: Option<Arc<StdMutex<DuckingParams>>>,
}

impl Default for InboundPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl InboundPipeline {
    pub fn new() -> Self {
        Self {
            capture: None,
            bridge: None,
            audio_mode: None,
            heartbeat: CaptureHeartbeat::new(),
            capture_device_id: None,
            playback_device: None,
            started_at_ms: None,
            capture_attach_tx: None,
            provider_tts: None,
            ducking_params: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.bridge.is_some()
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
        let Ok(resolved) = resolve_role_device(AudioRole::MeetingCapture, config, devices) else {
            return false;
        };
        self.capture_device_id.as_deref() == Some(resolved.id.as_str())
    }

    pub fn is_playback_healthy(&self, config: &AppConfig, devices: &[AudioDeviceInfo]) -> bool {
        if !config.inbound_mode.needs_playback() {
            return true;
        }
        let Ok(resolved) = resolve_role_device(AudioRole::LocalPlayback, config, devices) else {
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
            .ok_or_else(|| "Inbound pipeline is not running".to_string())?;
        handle.set_mode(mode)
    }

    pub(crate) fn provider_tts_runtime_mut(&mut self) -> Option<&mut InboundProviderTts> {
        self.provider_tts.as_mut()
    }

    /// Hot-apply ducked-original settings while Meeting → You translate is live.
    pub fn apply_ducking_params(&self, enabled: bool, gain: f32) {
        let Some(shared) = self.ducking_params.as_ref() else {
            return;
        };
        let next = DuckingParams::from_config_fields(enabled, gain);
        if let Ok(mut guard) = shared.lock() {
            *guard = next;
        }
    }

    pub fn apply_ducking_params_from_config(&self, config: &AppConfig) {
        self.apply_ducking_params(
            config.inbound_original_under_translation,
            config.inbound_original_ducked_gain,
        );
    }
}

pub(crate) fn unix_ms_now() -> u64 {
    crate::audio::monotonic_ms()
}
