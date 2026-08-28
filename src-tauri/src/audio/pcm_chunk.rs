//! Playback PCM chunk + 24 kHz adapter.
//!
//! A chunk is unmodified 24 kHz samples. The mux upsamples and plays them;
//! it does not rewrite or overlap-mix.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct PlaybackPcmChunk {
    pub samples: Vec<i16>,
}

impl PlaybackPcmChunk {
    pub fn new(samples: Vec<i16>) -> Self {
        Self { samples }
    }
}

/// Adapts legacy 24 kHz `Vec<i16>` bridge PCM into [`PlaybackPcmChunk`].
pub fn spawn_pcm24k_adapter(
    cancel: CancellationToken,
    mut pcm_rx: mpsc::Receiver<Vec<i16>>,
    pcm_drops: Arc<std::sync::atomic::AtomicU64>,
) -> mpsc::Receiver<PlaybackPcmChunk> {
    let (tx, rx) = mpsc::channel(super::PLAYBACK_PCM_CHANNEL_DEPTH);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                msg = pcm_rx.recv() => {
                    match msg {
                        Some(samples) => {
                            let _ = super::try_send_pcm_bounded(
                                &tx,
                                PlaybackPcmChunk::new(samples),
                                &pcm_drops,
                            );
                        }
                        None => break,
                    }
                }
            }
        }
    });
    rx
}
