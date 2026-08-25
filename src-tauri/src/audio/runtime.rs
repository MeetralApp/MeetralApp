use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex as StdMutex,
};

use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::ducking_mix::{DuckingMixer, DuckingParams, EnqueueTtsOutcome};
use super::pcm_channel::{try_send_pcm_bounded, PLAYBACK_PCM_CHANNEL_DEPTH};
use super::pcm_chunk::PlaybackPcmChunk;
use super::playback::{start_playback, PlaybackHandle};
use super::playback_buffer::PlaybackBufferConfig;
use super::resampler::upsample_24k_to_48k;
use super::ResolvedDevice;
use crate::config::PipelineOutputMode;
use crate::voice::config::VOICE_ENGINE_CUSTOM;

const MODE_TRANSLATED: u8 = 0;
const MODE_ORIGINAL: u8 = 1;
const MODE_TEXT: u8 = 2;

/// Hot-swappable playback target shared with [`spawn_pipeline_audio`].
pub type SharedPlaybackDevice = Arc<StdMutex<ResolvedDevice>>;

pub fn shared_playback_device(device: ResolvedDevice) -> SharedPlaybackDevice {
    Arc::new(StdMutex::new(device))
}

pub struct AudioModeHandle {
    mode: Arc<AtomicU8>,
    generation: Arc<AtomicU8>,
}

impl AudioModeHandle {
    pub fn new(mode: PipelineOutputMode) -> Self {
        Self {
            mode: Arc::new(AtomicU8::new(mode_to_atomic(mode))),
            generation: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn shared_mode(&self) -> Arc<AtomicU8> {
        self.mode.clone()
    }

    pub fn shared_generation(&self) -> Arc<AtomicU8> {
        self.generation.clone()
    }

    pub fn set_mode(&self, mode: PipelineOutputMode) -> Result<(), String> {
        self.mode.store(mode_to_atomic(mode), Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn current_mode(&self) -> PipelineOutputMode {
        atomic_to_mode(self.mode.load(Ordering::SeqCst))
    }
}

pub fn mode_to_atomic(mode: PipelineOutputMode) -> u8 {
    match mode {
        PipelineOutputMode::Translated => MODE_TRANSLATED,
        PipelineOutputMode::OriginalAudio => MODE_ORIGINAL,
        PipelineOutputMode::TextOnly => MODE_TEXT,
    }
}

pub fn atomic_to_mode(value: u8) -> PipelineOutputMode {
    match value {
        MODE_ORIGINAL => PipelineOutputMode::OriginalAudio,
        MODE_TEXT => PipelineOutputMode::TextOnly,
        _ => PipelineOutputMode::Translated,
    }
}

pub fn needs_playback_atomic(value: u8) -> bool {
    value == MODE_TRANSLATED || value == MODE_ORIGINAL
}

pub struct MicMuteHandle {
    muted: Arc<AtomicBool>,
}

impl Default for MicMuteHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl MicMuteHandle {
    pub fn new() -> Self {
        Self {
            muted: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn shared(&self) -> Arc<AtomicBool> {
        self.muted.clone()
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }
}

pub fn gate_pcm(muted: &AtomicBool, pcm: Vec<i16>) -> Vec<i16> {
    if muted.load(Ordering::Relaxed) {
        vec![0; pcm.len()]
    } else {
        pcm
    }
}

/// Zero-fill PCM in place when muted — avoids allocation on the hot path.
pub fn gate_pcm_in_place(muted: &AtomicBool, pcm: &mut [i16]) {
    if muted.load(Ordering::Relaxed) {
        pcm.fill(0);
    }
}

/// Spawn the playback mux task. Pass `app` for inbound underlay so capacity drops can
/// emit a debounced `inbound-tts-queue-drop` event for FE toasts.
pub fn spawn_pipeline_audio(
    cancel: CancellationToken,
    mode: Arc<AtomicU8>,
    generation: Arc<AtomicU8>,
    playback_device: SharedPlaybackDevice,
    mut bridge_audio_rx: mpsc::Receiver<PlaybackPcmChunk>,
    mut passthrough_rx: mpsc::Receiver<Vec<i16>>,
    playback_gate: Option<Arc<AtomicBool>>,
    bridge_ready: Option<Arc<AtomicBool>>,
    voice_engine: Option<Arc<AtomicU8>>,
    ducking_params: Option<Arc<StdMutex<DuckingParams>>>,
    app: Option<AppHandle>,
    pcm_drops: Arc<AtomicU64>,
) {
    tokio::spawn(async move {
        let mut last_generation = generation.load(Ordering::SeqCst);
        let mut playback: Option<PlaybackHandle> = None;
        let mut playback_tx: Option<mpsc::Sender<Vec<i16>>> = None;
        let mut active_device_id = String::new();
        let mut ducking = ducking_params.as_ref().map(|params| {
            let snap = params
                .lock()
                .map(|g| *g)
                .unwrap_or_else(|e| *e.into_inner());
            DuckingMixer::new(snap)
        });
        let mut last_tts_drop_toast_at =
            std::time::Instant::now() - std::time::Duration::from_secs(60);

        let notify_tts_drop = |last_at: &mut std::time::Instant| {
            let Some(app) = app.as_ref() else {
                return;
            };
            const DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(5);
            if last_at.elapsed() < DEBOUNCE {
                return;
            }
            *last_at = std::time::Instant::now();
            let _ = app.emit("inbound-tts-queue-drop", ());
        };

        let bridge_degraded = || match bridge_ready.as_ref() {
            Some(flag) => !flag.load(Ordering::Relaxed),
            None => false,
        };

        let sync_ducking_params = |mixer: &mut DuckingMixer| {
            if let Some(params) = ducking_params.as_ref() {
                let snap = params
                    .lock()
                    .map(|g| *g)
                    .unwrap_or_else(|e| *e.into_inner());
                if snap != mixer.params() {
                    mixer.set_params(snap);
                }
            }
        };

        let send_playback = |tx: &mpsc::Sender<Vec<i16>>, pcm: Vec<i16>| {
            let pcm = match playback_gate.as_ref() {
                Some(gate) => gate_pcm(gate, pcm),
                None => pcm,
            };
            let _ = try_send_pcm_bounded(tx, pcm, &pcm_drops);
        };

        let current_device = || -> ResolvedDevice {
            playback_device
                .lock()
                .map(|d| d.clone())
                .unwrap_or_else(|_| ResolvedDevice {
                    id: String::new(),
                    name: String::new(),
                    direction: "output",
                })
        };

        while !cancel.is_cancelled() {
            if generation.load(Ordering::SeqCst) != last_generation {
                last_generation = generation.load(Ordering::SeqCst);
                if let Some(handle) = playback.take() {
                    handle.stop();
                }
                playback_tx = None;
                active_device_id.clear();
                drain_chunks(&mut bridge_audio_rx);
                drain_bounded(&mut passthrough_rx);
                if let Some(m) = ducking.as_mut() {
                    m.reset();
                }
            }

            let current = mode.load(Ordering::SeqCst);
            let device = current_device();

            // Device hot-swap while already playing: bump closes stream above; reopen below.
            if playback.is_some() && !device.id.is_empty() && device.id != active_device_id {
                if let Some(handle) = playback.take() {
                    handle.stop();
                }
                playback_tx = None;
                active_device_id.clear();
                if let Some(m) = ducking.as_mut() {
                    m.reset();
                }
            }

            if needs_playback_atomic(current) && playback.is_none() {
                if device.id.is_empty() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    continue;
                }
                let (tx, rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
                match start_playback(device.clone(), rx, playback_buffer_config(&voice_engine)) {
                    Ok(handle) => {
                        active_device_id = device.id.clone();
                        playback = Some(handle);
                        playback_tx = Some(tx);
                    }
                    Err(e) => {
                        tracing::error!(
                            "failed to start playback for {} ({}): {e:#}",
                            device.name,
                            device.id
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        continue;
                    }
                }
            } else if !needs_playback_atomic(current) && playback.is_some() {
                if let Some(handle) = playback.take() {
                    handle.stop();
                }
                playback_tx = None;
                active_device_id.clear();
                drain_chunks(&mut bridge_audio_rx);
                drain_bounded(&mut passthrough_rx);
                if let Some(m) = ducking.as_mut() {
                    m.reset();
                }
            }

            match current {
                MODE_TRANSLATED => {
                    let Some(tx) = playback_tx.as_ref() else {
                        drain_chunks(&mut bridge_audio_rx);
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        continue;
                    };
                    if let Some(m) = ducking.as_mut() {
                        sync_ducking_params(m);
                    }
                    // Underlay playback is driven only by passthrough. Prefer that arm so
                    // biased select + merge_pending_chunks cannot starve the depth-6 clock.
                    let underlay_clock = underlay_uses_passthrough_clock(ducking.as_ref());
                    if underlay_clock {
                        tokio::select! {
                                                   biased;
                                                   _ = cancel.cancelled() => break,
                                                   pcm = passthrough_rx.recv() => {
                                                       match pcm {
                                                           Some(pcm_48k) if bridge_degraded() => {
                                                               drain_chunks(&mut bridge_audio_rx);
                                                               if let Some(m) = ducking.as_mut() {
                                                                   m.reset();
                                                               }
                                                               send_playback(tx, pcm_48k);
                                                           }
                                                           Some(pcm_48k) => {
                        // Ingest pending TTS *before* advancing the clock so
                        // passthrough-first select cannot underrun mid-utterance.
                                                               let (_ingested, dropped) = ingest_pending_underlay_tts(
                                                                   &mut bridge_audio_rx,
                                                                   &mut ducking,
                                                                   UNDERLAY_TTS_INGEST_BUDGET,
                                                                   UNDERLAY_TTS_INGEST_SAMPLES,
                                                               );
                                                               if dropped {
                                                                   notify_tts_drop(&mut last_tts_drop_toast_at);
                                                               }
                                                               if let Some(m) = ducking.as_mut() {
                                                                   sync_ducking_params(m);
                                                                   if let Some(mixed) =
                                                                       m.render_with_original(&pcm_48k)
                                                                   {
                                                                       send_playback(tx, mixed);
                                                                   }
                                                               }
                                                           }
                                                           None => break,
                                                       }
                                                   }
                                                   chunk = bridge_audio_rx.recv() => {
                                                       match chunk {
                                                           Some(chunk) if !bridge_degraded() => {
                                                               let pcm_48k = upsample_chunk(chunk);
                                                               if !pcm_48k.is_empty() {
                                                                   if let Some(m) = ducking.as_mut() {
                                                                       sync_ducking_params(m);
                                                                       if m.params().exclusive_tts_parity() {
                                                                           send_playback(tx, pcm_48k);
                                                                       } else if m.enqueue_tts(&pcm_48k)
                                                                           == EnqueueTtsOutcome::DroppedOverCap
                                                                       {
                                                                           notify_tts_drop(&mut last_tts_drop_toast_at);
                                                                       }
                                                                   }
                                                               }
                                                           }
                                                           Some(_) => {}
                                                           None => break,
                                                       }
                                                   }
                                               }
                    } else {
                        tokio::select! {
                                                   biased;
                                                   _ = cancel.cancelled() => break,
                                                   chunk = bridge_audio_rx.recv() => {
                                                       match chunk {
                                                           Some(chunk) if !bridge_degraded() => {
                                                               drain_bounded(&mut passthrough_rx);
                                                               let chunk =
                                                                   merge_pending_chunks(chunk, &mut bridge_audio_rx);
                                                               let pcm_48k = upsample_chunk(chunk);
                                                               if !pcm_48k.is_empty() {
                                                                   if let Some(m) = ducking.as_mut() {
                                                                       sync_ducking_params(m);
                                                                   }
                                                                   send_playback(tx, pcm_48k);
                                                               }
                                                           }
                                                           Some(_) => {}
                                                           None => break,
                                                       }
                                                   }
                                                   pcm = passthrough_rx.recv() => {
                                                       match pcm {
                                                           Some(pcm_48k) if bridge_degraded() => {
                                                               drain_chunks(&mut bridge_audio_rx);
                                                               if let Some(m) = ducking.as_mut() {
                                                                   m.reset();
                                                               }
                                                               send_playback(tx, pcm_48k);
                                                           }
                                                           Some(_pcm_48k) => {
                        // exclusive TTS: discard passthrough
                                                           }
                                                           None => break,
                                                       }
                                                   }
                                               }
                    }
                }
                MODE_ORIGINAL => {
                    let Some(tx) = playback_tx.as_ref() else {
                        drain_bounded(&mut passthrough_rx);
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        continue;
                    };
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        pcm = passthrough_rx.recv() => {
                            match pcm {
                                Some(pcm_48k) => {
                                    drain_chunks(&mut bridge_audio_rx);
                                    send_playback(tx, pcm_48k);
                                }
                                None => break,
                            }
                        }
                        chunk = bridge_audio_rx.recv() => {
                            let _ = chunk;
                        }
                    }
                }
                _ => {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        chunk = bridge_audio_rx.recv() => {
                            if chunk.is_none() {
                                break;
                            }
                        }
                        pcm = passthrough_rx.recv() => {
                            if pcm.is_none() {
                                break;
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(25)) => {}
                    }
                }
            }
        }

        if let Some(m) = ducking.as_mut() {
            m.reset();
        }
        if let Some(handle) = playback.take() {
            handle.stop();
        }
    });
}

fn playback_buffer_config(voice_engine: &Option<Arc<AtomicU8>>) -> PlaybackBufferConfig {
    if voice_engine
        .as_ref()
        .is_some_and(|engine| engine.load(Ordering::SeqCst) == VOICE_ENGINE_CUSTOM)
    {
        PlaybackBufferConfig::clone_outbound()
    } else {
        PlaybackBufferConfig::default()
    }
}

/// Continuous underlay uses passthrough as the sole playback clock; bridge only enqueues TTS.
fn underlay_uses_passthrough_clock(mixer: Option<&DuckingMixer>) -> bool {
    mixer.is_some_and(|m| !m.params().exclusive_tts_parity())
}

/// Max bridge chunks / upsampled samples to pull into the TTS queue per
/// passthrough frame. Keeps ingest ahead of the clock without blocking the
/// depth-6 passthrough path on huge backlogs.
const UNDERLAY_TTS_INGEST_BUDGET: usize = 32;
const UNDERLAY_TTS_INGEST_SAMPLES: usize = 48_000;

fn upsample_chunk(chunk: PlaybackPcmChunk) -> Vec<i16> {
    match upsample_24k_to_48k(&chunk.samples) {
        Ok(pcm) => pcm,
        Err(e) => {
            tracing::error!("audio resample: {e:#}");
            Vec::new()
        }
    }
}

/// Pull pending bridge TTS into the ducking queue before advancing the passthrough clock.
/// Without this, biased passthrough-first select starves enqueue and `tts_q` underruns
/// mid-utterance while later sub-chunks sit on `bridge_audio_rx`.
fn ingest_pending_underlay_tts(
    bridge_audio_rx: &mut mpsc::Receiver<PlaybackPcmChunk>,
    ducking: &mut Option<DuckingMixer>,
    chunk_budget: usize,
    sample_budget: usize,
) -> (usize, bool) {
    let mut ingested = 0usize;
    let mut samples = 0usize;
    let mut dropped_over_cap = false;
    while ingested < chunk_budget && samples < sample_budget {
        let Ok(chunk) = bridge_audio_rx.try_recv() else {
            break;
        };
        let pcm_48k = upsample_chunk(chunk);
        ingested += 1;
        if pcm_48k.is_empty() {
            continue;
        }
        samples = samples.saturating_add(pcm_48k.len());
        if let Some(m) = ducking.as_mut() {
            if m.params().exclusive_tts_parity() {
                break;
            }
            if m.enqueue_tts(&pcm_48k) == EnqueueTtsOutcome::DroppedOverCap {
                dropped_over_cap = true;
            }
        }
    }
    (ingested, dropped_over_cap)
}

fn merge_pending_chunks(
    mut first: PlaybackPcmChunk,
    rx: &mut mpsc::Receiver<PlaybackPcmChunk>,
) -> PlaybackPcmChunk {
    while let Ok(next) = rx.try_recv() {
        first.samples.extend(next.samples);
    }
    first
}

fn drain_chunks(rx: &mut mpsc::Receiver<PlaybackPcmChunk>) {
    while rx.try_recv().is_ok() {}
}

fn drain_bounded(rx: &mut mpsc::Receiver<Vec<i16>>) {
    while rx.try_recv().is_ok() {}
}

pub fn spawn_bridge_audio_drain(
    cancel: CancellationToken,
    mut bridge_audio_rx: mpsc::Receiver<Vec<i16>>,
) {
    tokio::spawn(async move {
        while !cancel.is_cancelled() {
            if bridge_audio_rx.recv().await.is_none() {
                break;
            }
        }
    });
}

pub fn send_passthrough(tx: &mpsc::Sender<Vec<i16>>, pcm: Vec<i16>) {
    if let Err(mpsc::error::TrySendError::Full(_)) = tx.try_send(pcm) {
        // Drop frame when passthrough buffer is full (prefer low latency).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_atomic_roundtrip() {
        assert_eq!(
            mode_to_atomic(PipelineOutputMode::Translated),
            MODE_TRANSLATED
        );
        assert_eq!(
            mode_to_atomic(PipelineOutputMode::OriginalAudio),
            MODE_ORIGINAL
        );
        assert_eq!(mode_to_atomic(PipelineOutputMode::TextOnly), MODE_TEXT);
        assert_eq!(
            atomic_to_mode(MODE_TRANSLATED),
            PipelineOutputMode::Translated
        );
        assert_eq!(
            atomic_to_mode(MODE_ORIGINAL),
            PipelineOutputMode::OriginalAudio
        );
        assert_eq!(atomic_to_mode(MODE_TEXT), PipelineOutputMode::TextOnly);
    }

    #[test]
    fn gate_pcm_returns_zeros_when_muted() {
        let muted = Arc::new(AtomicBool::new(true));
        let pcm = vec![1, 2, 3, -4];
        let gated = gate_pcm(&muted, pcm);
        assert_eq!(gated, vec![0, 0, 0, 0]);
    }

    #[test]
    fn gate_pcm_passes_through_when_unmuted() {
        let muted = Arc::new(AtomicBool::new(false));
        let pcm = vec![1, 2, 3, -4];
        let gated = gate_pcm(&muted, pcm.clone());
        assert_eq!(gated, pcm);
    }

    #[test]
    fn mic_mute_handle_tracks_state() {
        let handle = MicMuteHandle::new();
        assert!(!handle.is_muted());
        handle.set_muted(true);
        assert!(handle.is_muted());
        handle.set_muted(false);
        assert!(!handle.is_muted());
    }

    #[test]
    fn needs_playback_atomic_matches_modes() {
        assert!(needs_playback_atomic(MODE_TRANSLATED));
        assert!(needs_playback_atomic(MODE_ORIGINAL));
        assert!(!needs_playback_atomic(MODE_TEXT));
    }

    #[test]
    fn underlay_clock_when_enabled_with_gain() {
        let on = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.18));
        let off = DuckingMixer::new(DuckingParams::from_config_fields(false, 0.18));
        let zero = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.0));
        assert!(underlay_uses_passthrough_clock(Some(&on)));
        assert!(!underlay_uses_passthrough_clock(Some(&off)));
        assert!(!underlay_uses_passthrough_clock(Some(&zero)));
        assert!(!underlay_uses_passthrough_clock(None));
    }

    #[test]
    fn ingest_pending_underlay_tts_prevents_mid_utterance_underrun() {
        let (tx, mut rx) = mpsc::channel::<PlaybackPcmChunk>(PLAYBACK_PCM_CHANNEL_DEPTH);
        // Two 24 kHz sub-chunks of one utterance (upsample → 8 samples each @ 48 kHz).
        tx.try_send(PlaybackPcmChunk::new(vec![1000i16; 4]))
            .unwrap();
        tx.try_send(PlaybackPcmChunk::new(vec![2000i16; 4]))
            .unwrap();

        let mut ducking = Some(DuckingMixer::new(DuckingParams::from_config_fields(
            true, 0.18,
        )));
        let (ingested, dropped) = ingest_pending_underlay_tts(
            &mut rx,
            &mut ducking,
            UNDERLAY_TTS_INGEST_BUDGET,
            UNDERLAY_TTS_INGEST_SAMPLES,
        );
        assert_eq!(ingested, 2);
        assert!(!dropped);

        let m = ducking.as_mut().unwrap();
        let frame = 8; // first sub-chunk @ 48 kHz
        let a = m.render_with_original(&vec![0; frame]).unwrap();
        let b = m.render_with_original(&vec![0; frame]).unwrap();
        assert!(a.iter().any(|&s| s != 0));
        assert!(
            b.iter().any(|&s| s != 0),
            "second sub-chunk must already be in queue before clock advances"
        );
    }

    #[tokio::test]
    async fn send_passthrough_drops_when_channel_full() {
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(vec![1]).unwrap();
        send_passthrough(&tx, vec![2, 3]);
        assert_eq!(rx.recv().await, Some(vec![1]));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn send_passthrough_accepts_when_channel_has_capacity() {
        let (tx, mut rx) = mpsc::channel(2);
        send_passthrough(&tx, vec![1]);
        send_passthrough(&tx, vec![2]);
        assert_eq!(rx.recv().await, Some(vec![1]));
        assert_eq!(rx.recv().await, Some(vec![2]));
    }

    #[test]
    fn audio_mode_handle_set_mode() {
        let handle = AudioModeHandle::new(PipelineOutputMode::Translated);
        assert!(handle.set_mode(PipelineOutputMode::TextOnly).is_ok());
        assert_eq!(handle.current_mode(), PipelineOutputMode::TextOnly);
        handle.set_mode(PipelineOutputMode::OriginalAudio).unwrap();
        assert_eq!(handle.current_mode(), PipelineOutputMode::OriginalAudio);
    }
}
