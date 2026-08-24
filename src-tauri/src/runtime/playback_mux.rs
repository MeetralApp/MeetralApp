use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc,
};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::audio::pcm_crossfade::PlaybackPcmChunk;
use crate::audio::runtime::atomic_to_mode;
use crate::audio::try_send_pcm_bounded;
use crate::capabilities::PlaybackSource;
use crate::config::PipelineOutputMode;
use crate::pipeline::drain::drain_unbounded;
use crate::runtime::voice_runtime::VOICE_ENGINE_CUSTOM;

async fn watch_mux_generation(gen: &AtomicU64, observed: u64) {
    loop {
        if gen.load(Ordering::SeqCst) != observed {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }
}

/// Routes bridge / provider-TTS / clone PCM at 24 kHz via [`PlaybackSource`].
pub fn spawn_outbound_playback_mux(
    cancel: CancellationToken,
    audio_mode: Arc<AtomicU8>,
    voice_engine: Arc<AtomicU8>,
    mux_generation: Arc<AtomicU64>,
    uses_separate_tts: bool,
    mut bridge_pcm_rx: mpsc::Receiver<Vec<i16>>,
    mut provider_tts_pcm_rx: mpsc::Receiver<PlaybackPcmChunk>,
    mut custom_pcm_rx: mpsc::Receiver<PlaybackPcmChunk>,
    playback_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Once a sender is dropped, `recv` returns None forever — disable that arm
        // instead of exiting the whole mux (provider TTS / clone may still be live).
        let mut bridge_open = true;
        let mut provider_tts_open = true;
        let mut custom_open = true;

        loop {
            if cancel.is_cancelled() {
                break;
            }
            if !bridge_open && !provider_tts_open && !custom_open {
                break;
            }

            let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
            if mode != PipelineOutputMode::Translated {
                if bridge_open {
                    drain_unbounded(&mut bridge_pcm_rx);
                }
                if provider_tts_open {
                    drain_unbounded(&mut provider_tts_pcm_rx);
                }
                if custom_open {
                    drain_unbounded(&mut custom_pcm_rx);
                }
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    msg = bridge_pcm_rx.recv(), if bridge_open => {
                        if msg.is_none() {
                            bridge_open = false;
                        }
                    }
                    msg = provider_tts_pcm_rx.recv(), if provider_tts_open => {
                        if msg.is_none() {
                            provider_tts_open = false;
                        }
                    }
                    msg = custom_pcm_rx.recv(), if custom_open => {
                        if msg.is_none() {
                            custom_open = false;
                        }
                    }
                }
                continue;
            }

            let gen = mux_generation.load(Ordering::SeqCst);
            let engine = voice_engine.load(Ordering::SeqCst);
            let source = if engine == VOICE_ENGINE_CUSTOM {
                PlaybackSource::CustomTts
            } else if uses_separate_tts {
                PlaybackSource::ProviderTts
            } else {
                PlaybackSource::BridgeSts
            };

            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                pcm = bridge_pcm_rx.recv(), if bridge_open => {
                    match pcm {
                        Some(pcm_24k) => {
                            if source == PlaybackSource::BridgeSts {
                                let _ = try_send_pcm_bounded(
                                    &playback_tx,
                                    PlaybackPcmChunk::continuation(pcm_24k),
                                    &pcm_drops,
                                );
                            }
                        }
                        None => {
                            bridge_open = false;
                            info!("outbound mux: bridge PCM channel closed");
                        }
                    }
                }
                pcm = provider_tts_pcm_rx.recv(), if provider_tts_open => {
                    match pcm {
                        Some(chunk) => {
                            if source == PlaybackSource::ProviderTts {
                                let _ = try_send_pcm_bounded(&playback_tx, chunk, &pcm_drops);
                            }
                        }
                        None => {
                            provider_tts_open = false;
                            info!("outbound mux: provider TTS PCM channel closed");
                        }
                    }
                }
                pcm = custom_pcm_rx.recv(), if custom_open => {
                    match pcm {
                        Some(chunk) => {
                            if source == PlaybackSource::CustomTts {
                                let _ = try_send_pcm_bounded(&playback_tx, chunk, &pcm_drops);
                            }
                        }
                        None => {
                            custom_open = false;
                            info!("outbound mux: clone PCM channel closed");
                        }
                    }
                }
                _ = watch_mux_generation(&mux_generation, gen) => {
                    let drained_bridge = if bridge_open {
                        drain_unbounded(&mut bridge_pcm_rx)
                    } else {
                        0
                    };
                    let drained_provider = if provider_tts_open {
                        drain_unbounded(&mut provider_tts_pcm_rx)
                    } else {
                        0
                    };
                    let drained_custom = if custom_open {
                        drain_unbounded(&mut custom_pcm_rx)
                    } else {
                        0
                    };
                    if drained_bridge > 0 || drained_provider > 0 || drained_custom > 0 {
                        info!(
                            bridge_chunks = drained_bridge,
                            provider_tts_chunks = drained_provider,
                            clone_chunks = drained_custom,
                            "mux generation bump — drained PCM queues"
                        );
                    }
                }
            }
        }
        info!("outbound playback mux stopped");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, AtomicU8};

    use crate::audio::runtime::mode_to_atomic;
    use crate::audio::PLAYBACK_PCM_CHANNEL_DEPTH;
    use crate::runtime::voice_runtime::VOICE_ENGINE_PROVIDER;

    fn drops() -> Arc<AtomicU64> {
        Arc::new(AtomicU64::new(0))
    }

    #[tokio::test]
    async fn mux_routes_provider_only() {
        let cancel = CancellationToken::new();
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_PROVIDER));
        let mux_generation = Arc::new(AtomicU64::new(0));

        let (bridge_tx, bridge_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_provider_tx, provider_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_clone_tx, clone_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (playback_tx, mut playback_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);

        let handle = spawn_outbound_playback_mux(
            cancel.clone(),
            audio_mode,
            voice_engine,
            mux_generation,
            false,
            bridge_rx,
            provider_rx,
            clone_rx,
            playback_tx,
            drops(),
        );

        bridge_tx.send(vec![1000i16; 240]).await.unwrap();

        let got = tokio::time::timeout(tokio::time::Duration::from_millis(500), playback_rx.recv())
            .await
            .ok()
            .flatten();

        cancel.cancel();
        let _ = handle.await;

        assert_eq!(got.as_ref().map(|c| c.samples.len()), Some(240));
    }

    #[tokio::test]
    async fn mux_routes_provider_tts() {
        let cancel = CancellationToken::new();
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_PROVIDER));
        let mux_generation = Arc::new(AtomicU64::new(0));

        let (_bridge_tx, bridge_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (provider_tx, provider_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_clone_tx, clone_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (playback_tx, mut playback_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);

        let handle = spawn_outbound_playback_mux(
            cancel.clone(),
            audio_mode,
            voice_engine,
            mux_generation,
            true,
            bridge_rx,
            provider_rx,
            clone_rx,
            playback_tx,
            drops(),
        );

        provider_tx
            .send(PlaybackPcmChunk::continuation(vec![500i16; 120]))
            .await
            .unwrap();

        let got = tokio::time::timeout(tokio::time::Duration::from_millis(500), playback_rx.recv())
            .await
            .ok()
            .flatten();

        cancel.cancel();
        let _ = handle.await;

        assert_eq!(got.as_ref().map(|c| c.samples.len()), Some(120));
    }

    #[tokio::test]
    async fn mux_survives_bridge_pcm_closed_for_provider_tts() {
        let cancel = CancellationToken::new();
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_PROVIDER));
        let mux_generation = Arc::new(AtomicU64::new(0));

        let (bridge_tx, bridge_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (provider_tx, provider_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_clone_tx, clone_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (playback_tx, mut playback_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);

        let handle = spawn_outbound_playback_mux(
            cancel.clone(),
            audio_mode,
            voice_engine,
            mux_generation,
            true,
            bridge_rx,
            provider_rx,
            clone_rx,
            playback_tx,
            drops(),
        );

        // Simulate Soniox STT dropping unused bridge audio sender immediately.
        drop(bridge_tx);
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        provider_tx
            .send(PlaybackPcmChunk::continuation(vec![700i16; 96]))
            .await
            .unwrap();

        let got = tokio::time::timeout(tokio::time::Duration::from_millis(500), playback_rx.recv())
            .await
            .ok()
            .flatten();

        cancel.cancel();
        let _ = handle.await;

        assert_eq!(got.as_ref().map(|c| c.samples.len()), Some(96));
    }

    #[tokio::test]
    async fn mux_routes_custom_tts() {
        let cancel = CancellationToken::new();
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_CUSTOM));
        let mux_generation = Arc::new(AtomicU64::new(0));

        let (_bridge_tx, bridge_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_provider_tx, provider_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (clone_tx, clone_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (playback_tx, mut playback_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);

        let handle = spawn_outbound_playback_mux(
            cancel.clone(),
            audio_mode,
            voice_engine,
            mux_generation,
            false,
            bridge_rx,
            provider_rx,
            clone_rx,
            playback_tx,
            drops(),
        );

        clone_tx
            .send(PlaybackPcmChunk::continuation(vec![300i16; 80]))
            .await
            .unwrap();

        let got = tokio::time::timeout(tokio::time::Duration::from_millis(500), playback_rx.recv())
            .await
            .ok()
            .flatten();

        cancel.cancel();
        let _ = handle.await;

        assert_eq!(got.as_ref().map(|c| c.samples.len()), Some(80));
    }

    #[tokio::test]
    async fn mux_drops_bridge_pcm_when_separate_tts() {
        let cancel = CancellationToken::new();
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_PROVIDER));
        let mux_generation = Arc::new(AtomicU64::new(0));

        let (bridge_tx, bridge_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (provider_tx, provider_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_clone_tx, clone_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (playback_tx, mut playback_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);

        let handle = spawn_outbound_playback_mux(
            cancel.clone(),
            audio_mode,
            voice_engine,
            mux_generation,
            true,
            bridge_rx,
            provider_rx,
            clone_rx,
            playback_tx,
            drops(),
        );

        bridge_tx.send(vec![1000i16; 240]).await.unwrap();
        let leaked =
            tokio::time::timeout(tokio::time::Duration::from_millis(80), playback_rx.recv())
                .await
                .ok()
                .flatten();
        assert!(
            leaked.is_none(),
            "bridge PCM must not play under separate TTS"
        );

        provider_tx
            .send(PlaybackPcmChunk::continuation(vec![500i16; 64]))
            .await
            .unwrap();
        let got = tokio::time::timeout(tokio::time::Duration::from_millis(500), playback_rx.recv())
            .await
            .ok()
            .flatten();

        cancel.cancel();
        let _ = handle.await;

        assert_eq!(got.as_ref().map(|c| c.samples.len()), Some(64));
    }

    #[tokio::test]
    async fn mux_drains_without_playback_when_not_translated() {
        let cancel = CancellationToken::new();
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(PipelineOutputMode::TextOnly)));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_PROVIDER));
        let mux_generation = Arc::new(AtomicU64::new(0));

        let (bridge_tx, bridge_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (_provider_tx, provider_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (clone_tx, clone_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (playback_tx, mut playback_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);

        let handle = spawn_outbound_playback_mux(
            cancel.clone(),
            audio_mode,
            voice_engine,
            mux_generation,
            false,
            bridge_rx,
            provider_rx,
            clone_rx,
            playback_tx,
            drops(),
        );

        bridge_tx.send(vec![1000i16; 120]).await.unwrap();
        clone_tx
            .send(PlaybackPcmChunk::continuation(vec![300i16; 40]))
            .await
            .unwrap();

        let leaked =
            tokio::time::timeout(tokio::time::Duration::from_millis(80), playback_rx.recv())
                .await
                .ok()
                .flatten();
        assert!(
            leaked.is_none(),
            "non-Translated mode must not emit playback PCM"
        );

        cancel.cancel();
        let _ = handle.await;
    }
}
