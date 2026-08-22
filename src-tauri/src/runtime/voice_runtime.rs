use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::audio::pcm_crossfade::PlaybackPcmChunk;
use crate::audio::runtime::atomic_to_mode;
use crate::capabilities::{
    bridge_play_audio_enabled, live_caps,
    tts_text_pipeline_active as routing_tts_text_pipeline_active,
};
use crate::config::{AppConfig, OutboundVoiceOutput, PipelineOutputMode};
use crate::runtime::control_channel;
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::{
    spawn_elevenlabs_tts_worker, spawn_soniox_tts_worker, ElevenLabsWorkerConfig,
    SonioxTtsWorkerConfig, TtsTextCommand, VoiceTtsStatus,
};

/// Provider native TTS = 0, ElevenLabs clone = 1
pub type VoiceEngineAtomic = Arc<AtomicU8>;

pub use crate::voice::config::{VOICE_ENGINE_CLONE, VOICE_ENGINE_PROVIDER};

const EL_READY_TIMEOUT: Duration = Duration::from_secs(15);
const EL_JOIN_TIMEOUT: Duration = Duration::from_secs(3);
const VOICE_SWITCH_COOLDOWN: Duration = Duration::from_millis(800);
const EL_IDLE_CLOSE: Duration = Duration::from_secs(60);

pub fn voice_output_to_engine(output: OutboundVoiceOutput) -> u8 {
    match output {
        OutboundVoiceOutput::ProviderNative => VOICE_ENGINE_PROVIDER,
        OutboundVoiceOutput::ElevenLabsClone => VOICE_ENGINE_CLONE,
    }
}

pub fn engine_to_voice_output(engine: u8) -> OutboundVoiceOutput {
    if engine == VOICE_ENGINE_CLONE {
        OutboundVoiceOutput::ElevenLabsClone
    } else {
        OutboundVoiceOutput::ProviderNative
    }
}

/// Text→TTS pipeline is active for ElevenLabs clone, or Soniox Provider (separate TTS).
pub fn tts_active_for_engine(
    mode: PipelineOutputMode,
    engine: u8,
    switch_in_progress: bool,
) -> bool {
    !switch_in_progress && mode == PipelineOutputMode::Translated && engine == VOICE_ENGINE_CLONE
}

pub fn tts_text_pipeline_active(
    mode: PipelineOutputMode,
    engine: u8,
    switch_in_progress: bool,
    uses_separate_tts: bool,
) -> bool {
    routing_tts_text_pipeline_active(mode, engine, switch_in_progress, uses_separate_tts)
}

pub struct OutboundTtsSession {
    pub worker: JoinHandle<()>,
    pub worker_cancel: CancellationToken,
    pub forward_cancel: CancellationToken,
}

pub struct OutboundVoiceRuntime {
    pub voice_engine: VoiceEngineAtomic,
    pub bridge_play_audio: Arc<AtomicBool>,
    pub audio_mode: Arc<AtomicU8>,
    pub mux_generation: Arc<AtomicU64>,
    pub voice_switch_mutex: Arc<tokio::sync::Mutex<()>>,
    pub voice_switch_in_progress: Arc<AtomicBool>,
    pub last_switch_at: Arc<StdMutex<Instant>>,
    pub relay_chars_while_provider: Arc<AtomicU64>,
    pub el_parent_cancel: CancellationToken,
    pub tts_cmd_tx: Arc<StdMutex<mpsc::Sender<TtsTextCommand>>>,
    pub clone_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    pub provider_tts_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    /// From live caps — provider uses a separate text→TTS WebSocket.
    pub uses_separate_tts: bool,
    /// From live caps — live bridge emits STS playback PCM.
    pub bridge_emits_playback_audio: bool,
    /// ElevenLabs clone worker only.
    pub el_worker: Mutex<Option<OutboundTtsSession>>,
    /// Soniox (or other provider-native) text→TTS worker.
    pub provider_tts_worker: Mutex<Option<OutboundTtsSession>>,
    pub voice_latency_tx: Option<mpsc::Sender<crate::voice::types::VoiceCloneLatencyEvent>>,
    pub turn_latency: Arc<TurnLatencySlot>,
    /// Shared with `AudioModeHandle` — bump to stop WASAPI/CoreAudio and drain PCM queues.
    pub playback_generation: Arc<AtomicU8>,
    /// Shared overflow counter for bounded PCM queues (engine `pcm_frames_dropped`).
    pub pcm_drops: Arc<AtomicU64>,
}

impl OutboundVoiceRuntime {
    pub fn new(
        config: &AppConfig,
        audio_mode: Arc<AtomicU8>,
        el_parent_cancel: CancellationToken,
        clone_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
        provider_tts_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
        voice_latency_tx: Option<mpsc::Sender<crate::voice::types::VoiceCloneLatencyEvent>>,
        playback_generation: Arc<AtomicU8>,
        pcm_drops: Arc<AtomicU64>,
    ) -> Arc<Self> {
        let (tts_cmd_tx, _) = mpsc::channel(control_channel::TTS_CMD_CHANNEL_DEPTH);
        let caps = live_caps(config.ai_provider);
        let runtime = Arc::new(Self {
            voice_engine: Arc::new(AtomicU8::new(voice_output_to_engine(
                config.outbound_voice_output,
            ))),
            bridge_play_audio: Arc::new(AtomicBool::new(false)),
            audio_mode,
            mux_generation: Arc::new(AtomicU64::new(0)),
            voice_switch_mutex: Arc::new(tokio::sync::Mutex::new(())),
            voice_switch_in_progress: Arc::new(AtomicBool::new(false)),
            last_switch_at: Arc::new(StdMutex::new(Instant::now())),
            relay_chars_while_provider: Arc::new(AtomicU64::new(0)),
            el_parent_cancel,
            tts_cmd_tx: Arc::new(StdMutex::new(tts_cmd_tx)),
            clone_pcm_tx,
            provider_tts_pcm_tx,
            uses_separate_tts: caps.uses_separate_tts,
            bridge_emits_playback_audio: caps.bridge_emits_playback_audio,
            el_worker: Mutex::new(None),
            provider_tts_worker: Mutex::new(None),
            voice_latency_tx,
            turn_latency: TurnLatencySlot::new_shared(),
            playback_generation,
            pcm_drops,
        });
        sync_bridge_play_audio(&runtime);
        runtime
    }

    pub fn bump_mux_generation(&self) {
        self.mux_generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Stop device playback immediately and drain in-flight PCM (G-STAB-5).
    pub fn flush_playback(&self) {
        self.playback_generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn cooldown_remaining(&self) -> Option<Duration> {
        let last = *crate::meeting::lock_poison_recover(&self.last_switch_at, "voice switch clock");
        let elapsed = last.elapsed();
        if elapsed < VOICE_SWITCH_COOLDOWN {
            Some(VOICE_SWITCH_COOLDOWN - elapsed)
        } else {
            None
        }
    }

    pub fn send_tts_cmd(&self, cmd: TtsTextCommand) {
        if let Ok(tx) = self.tts_cmd_tx.lock() {
            control_channel::try_send_control(&tx, cmd, "tts-cmd");
        }
    }
}

pub fn sync_bridge_play_audio(runtime: &OutboundVoiceRuntime) {
    let mode = atomic_to_mode(runtime.audio_mode.load(Ordering::Relaxed));
    let engine = runtime.voice_engine.load(Ordering::Relaxed);
    let play = bridge_play_audio_enabled(mode, engine, runtime.bridge_emits_playback_audio);
    runtime.bridge_play_audio.store(play, Ordering::SeqCst);
}

pub async fn ensure_el_worker(
    runtime: &Arc<OutboundVoiceRuntime>,
    config: &AppConfig,
    app_status_tx: Option<mpsc::Sender<VoiceTtsStatus>>,
) -> Result<(), String> {
    stop_provider_tts_worker(runtime).await;
    if runtime.el_worker.lock().await.is_some() {
        stop_el_worker(runtime).await;
    }

    let (tts_cmd_tx, tts_cmd_rx) = mpsc::channel(control_channel::TTS_CMD_CHANNEL_DEPTH);
    let (worker_status_tx, worker_status_rx) =
        mpsc::channel(control_channel::TTS_STATUS_CHANNEL_DEPTH);

    {
        let mut guard = crate::meeting::lock_poison_recover(&runtime.tts_cmd_tx, "tts cmd tx");
        *guard = tts_cmd_tx;
    }

    let worker_cancel = runtime.el_parent_cancel.child_token();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let mut ready_tx = Some(ready_tx);

    let forward_cancel = worker_cancel.child_token();
    let app_status = app_status_tx.clone();
    tokio::spawn(async move {
        let mut rx = worker_status_rx;
        let mut ready_signaled = false;
        while let Some(status) = rx.recv().await {
            let is_ready = matches!(status, VoiceTtsStatus::Ready);
            let degraded = if let VoiceTtsStatus::Degraded { ref message } = status {
                Some(message.clone())
            } else {
                None
            };
            if let Some(ref app_tx) = app_status {
                control_channel::try_send_control(app_tx, status, "tts-status");
            }
            if is_ready && !ready_signaled {
                ready_signaled = true;
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(Ok(()));
                }
            } else if let Some(msg) = degraded {
                if !ready_signaled {
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(Err(msg));
                    }
                    return;
                }
            }
        }
        if !ready_signaled {
            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(Err("ElevenLabs worker stopped unexpectedly".to_string()));
            }
        }
    });

    let worker = spawn_elevenlabs_tts_worker(
        ElevenLabsWorkerConfig {
            api_key: config.elevenlabs.elevenlabs_api_key.clone(),
            voice_id: config.elevenlabs.elevenlabs_voice_id.clone(),
            model_id: config.elevenlabs.elevenlabs_tts_model.clone(),
            init_settings: config.elevenlabs_init_settings(),
            language_code: config.resolve_elevenlabs_tts_language_code(),
            auto_mode: config.elevenlabs.elevenlabs_auto_mode,
        },
        tts_cmd_rx,
        runtime.clone_pcm_tx.clone(),
        runtime.pcm_drops.clone(),
        worker_status_tx,
        runtime.turn_latency.clone(),
        worker_cancel.clone(),
    );

    let ready = tokio::time::timeout(EL_READY_TIMEOUT, ready_rx).await;

    match ready {
        Ok(Ok(Ok(()))) => {
            *runtime.el_worker.lock().await = Some(OutboundTtsSession {
                worker,
                worker_cancel,
                forward_cancel,
            });
            info!("elevenlabs worker ready for outbound clone");
            Ok(())
        }
        Ok(Ok(Err(msg))) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(EL_JOIN_TIMEOUT, worker).await;
            Err(msg)
        }
        Ok(Err(_)) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(EL_JOIN_TIMEOUT, worker).await;
            Err("ElevenLabs worker stopped unexpectedly".to_string())
        }
        Err(_) => {
            worker_cancel.cancel();
            forward_cancel.cancel();
            let _ = tokio::time::timeout(EL_JOIN_TIMEOUT, worker).await;
            Err("ElevenLabs connect timeout (15s)".to_string())
        }
    }
}

/// Start provider-native TTS worker for Provider voice (feeds `provider_tts_pcm_tx`).
pub async fn ensure_provider_tts_worker(
    runtime: &Arc<OutboundVoiceRuntime>,
    config: &AppConfig,
    language: &str,
    app_status_tx: Option<mpsc::Sender<VoiceTtsStatus>>,
) -> Result<(), String> {
    stop_el_worker(runtime).await;
    if runtime.provider_tts_worker.lock().await.is_some() {
        stop_provider_tts_worker(runtime).await;
    }

    let (tts_cmd_tx, tts_cmd_rx) = mpsc::channel(control_channel::TTS_CMD_CHANNEL_DEPTH);
    let (worker_status_tx, worker_status_rx) =
        mpsc::channel(control_channel::TTS_STATUS_CHANNEL_DEPTH);

    {
        let mut guard = crate::meeting::lock_poison_recover(&runtime.tts_cmd_tx, "tts cmd tx");
        *guard = tts_cmd_tx;
    }

    let worker_cancel = runtime.el_parent_cancel.child_token();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let mut ready_tx = Some(ready_tx);

    let forward_cancel = worker_cancel.child_token();
    let app_status = app_status_tx.clone();
    tokio::spawn(async move {
        let mut rx = worker_status_rx;
        let mut ready_signaled = false;
        while let Some(status) = rx.recv().await {
            let is_ready = matches!(status, VoiceTtsStatus::Ready);
            let degraded = if let VoiceTtsStatus::Degraded { ref message } = status {
                Some(message.clone())
            } else {
                None
            };
            if let Some(ref app_tx) = app_status {
                control_channel::try_send_control(app_tx, status, "tts-status");
            }
            if is_ready && !ready_signaled {
                ready_signaled = true;
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(Ok(()));
                }
            } else if let Some(msg) = degraded {
                if !ready_signaled {
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(Err(msg));
                    }
                    return;
                }
            }
        }
        if !ready_signaled {
            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(Err("Soniox TTS worker stopped unexpectedly".to_string()));
            }
        }
    });

    let worker = spawn_soniox_tts_worker(
        SonioxTtsWorkerConfig {
            api_key: config.soniox_api_key.clone(),
            voice: config.soniox.soniox_tts_outbound_voice.clone(),
            model: config.soniox.soniox_tts_model.clone(),
            language: language.to_string(),
            speed: config.soniox.soniox_tts_outbound_speed,
        },
        tts_cmd_rx,
        runtime.provider_tts_pcm_tx.clone(),
        runtime.pcm_drops.clone(),
        worker_status_tx,
        runtime.turn_latency.clone(),
        worker_cancel.clone(),
    );

    let ready = tokio::time::timeout(EL_READY_TIMEOUT, ready_rx).await;

    match ready {
        Ok(Ok(Ok(()))) => {
            *runtime.provider_tts_worker.lock().await = Some(OutboundTtsSession {
                worker,
                worker_cancel,
                forward_cancel,
            });
            info!("soniox tts worker ready");
            Ok(())
        }
        Ok(Ok(Err(msg))) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(EL_JOIN_TIMEOUT, worker).await;
            Err(msg)
        }
        Ok(Err(_)) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(EL_JOIN_TIMEOUT, worker).await;
            Err("Soniox TTS worker stopped unexpectedly".to_string())
        }
        Err(_) => {
            worker_cancel.cancel();
            forward_cancel.cancel();
            let _ = tokio::time::timeout(EL_JOIN_TIMEOUT, worker).await;
            Err("Soniox TTS connect timeout (15s)".to_string())
        }
    }
}

pub(crate) async fn stop_tts_session(session: OutboundTtsSession, label: &str) {
    session.forward_cancel.cancel();
    session.worker_cancel.cancel();

    match tokio::time::timeout(EL_JOIN_TIMEOUT, session.worker).await {
        Ok(Ok(())) => info!("{label} worker stopped cleanly"),
        Ok(Err(e)) => warn!("{label} worker join error: {e}"),
        Err(_) => warn!("{label} worker join timed out — force dropped"),
    }
}

pub async fn stop_el_worker(runtime: &Arc<OutboundVoiceRuntime>) {
    let session = runtime.el_worker.lock().await.take();
    let Some(session) = session else {
        return;
    };

    runtime.send_tts_cmd(TtsTextCommand::Flush);
    runtime.send_tts_cmd(TtsTextCommand::Reset);
    stop_tts_session(session, "elevenlabs").await;
}

pub async fn stop_provider_tts_worker(runtime: &Arc<OutboundVoiceRuntime>) {
    let session = runtime.provider_tts_worker.lock().await.take();
    let Some(session) = session else {
        return;
    };

    runtime.send_tts_cmd(TtsTextCommand::Flush);
    runtime.send_tts_cmd(TtsTextCommand::Reset);
    stop_tts_session(session, "provider tts").await;
}

pub async fn stop_all_tts_workers(runtime: &Arc<OutboundVoiceRuntime>) {
    stop_el_worker(runtime).await;
    stop_provider_tts_worker(runtime).await;
}

/// Idle-close ElevenLabs only. Soniox Provider TTS stays up while outbound Translated.
pub fn spawn_el_idle_watcher(
    runtime: Arc<OutboundVoiceRuntime>,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut idle_since: Option<Instant> = None;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
            if cancel.is_cancelled() {
                break;
            }

            let mode = atomic_to_mode(runtime.audio_mode.load(Ordering::SeqCst));
            let engine = runtime.voice_engine.load(Ordering::SeqCst);
            let in_progress = runtime.voice_switch_in_progress.load(Ordering::SeqCst);

            // Clone path still active → keep ElevenLabs warm.
            if tts_active_for_engine(mode, engine, in_progress) {
                idle_since = None;
                continue;
            }

            if runtime.el_worker.lock().await.is_none() {
                idle_since = None;
                continue;
            }

            let now = Instant::now();
            if idle_since.is_none() {
                idle_since = Some(now);
            } else if now.duration_since(idle_since.unwrap()) >= EL_IDLE_CLOSE {
                info!("elevenlabs idle auto-close after 60s");
                stop_el_worker(&runtime).await;
                idle_since = None;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::runtime::mode_to_atomic;

    #[test]
    fn sync_bridge_play_audio_provider_translated() {
        let cancel = CancellationToken::new();
        let (tx, _) = mpsc::channel(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
        let (provider_tx, _) = mpsc::channel(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let playback_generation = Arc::new(AtomicU8::new(0));
        let config = AppConfig {
            outbound_voice_output: OutboundVoiceOutput::ProviderNative,
            ..AppConfig::default()
        };
        let runtime = OutboundVoiceRuntime::new(
            &config,
            audio_mode,
            cancel,
            tx,
            provider_tx,
            None,
            playback_generation,
            Arc::new(AtomicU64::new(0)),
        );
        assert!(runtime.bridge_play_audio.load(Ordering::SeqCst));

        runtime
            .voice_engine
            .store(VOICE_ENGINE_CLONE, Ordering::SeqCst);
        sync_bridge_play_audio(&runtime);
        assert!(!runtime.bridge_play_audio.load(Ordering::SeqCst));
    }

    #[test]
    fn tts_active_only_clone_translated() {
        assert!(tts_active_for_engine(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_CLONE,
            false
        ));
        assert!(!tts_active_for_engine(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_PROVIDER,
            false
        ));
        assert!(!tts_active_for_engine(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_CLONE,
            true
        ));
        assert!(!tts_active_for_engine(
            PipelineOutputMode::TextOnly,
            VOICE_ENGINE_CLONE,
            false
        ));
    }

    #[test]
    fn tts_text_pipeline_active_soniox_provider() {
        assert!(tts_text_pipeline_active(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_PROVIDER,
            false,
            true, // separate TTS
        ));
        assert!(!tts_text_pipeline_active(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_PROVIDER,
            false,
            false, // STS bridge providers
        ));
        assert!(!tts_text_pipeline_active(
            PipelineOutputMode::TextOnly,
            VOICE_ENGINE_PROVIDER,
            false,
            true,
        ));
        assert!(tts_text_pipeline_active(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_CLONE,
            false,
            true,
        ));
    }

    #[test]
    fn voice_output_engine_roundtrip() {
        assert_eq!(
            voice_output_to_engine(OutboundVoiceOutput::ProviderNative),
            VOICE_ENGINE_PROVIDER
        );
        assert_eq!(
            voice_output_to_engine(OutboundVoiceOutput::ElevenLabsClone),
            VOICE_ENGINE_CLONE
        );
        assert_eq!(
            engine_to_voice_output(VOICE_ENGINE_CLONE),
            OutboundVoiceOutput::ElevenLabsClone
        );
        assert_eq!(
            engine_to_voice_output(VOICE_ENGINE_PROVIDER),
            OutboundVoiceOutput::ProviderNative
        );
        assert_eq!(
            engine_to_voice_output(99),
            OutboundVoiceOutput::ProviderNative
        );
    }

    #[test]
    fn cooldown_remaining_tracks_last_switch() {
        let cancel = CancellationToken::new();
        let (tx, _) = mpsc::channel(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
        let (provider_tx, _) = mpsc::channel(crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH);
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let playback_generation = Arc::new(AtomicU8::new(0));
        let runtime = OutboundVoiceRuntime::new(
            &AppConfig::default(),
            audio_mode,
            cancel,
            tx,
            provider_tx,
            None,
            playback_generation,
            Arc::new(AtomicU64::new(0)),
        );

        *runtime.last_switch_at.lock().unwrap() = Instant::now();
        assert!(runtime.cooldown_remaining().is_some());

        *runtime.last_switch_at.lock().unwrap() =
            Instant::now() - VOICE_SWITCH_COOLDOWN - Duration::from_millis(50);
        assert!(runtime.cooldown_remaining().is_none());
    }
}
