use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use anyhow::{anyhow, Result};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

/// Bounded depth for microphone → bridge upload. At 100 ms frames this holds ~5 s of audio.
pub const AUDIO_IN_CHANNEL_DEPTH: usize = 50;

/// Wait for provider close flush (Soniox finished wait / OpenAI session.closed).
/// Keep slightly above Soniox `FINISHED_WAIT` so graceful close can finish; abort after.
const BRIDGE_STOP_JOIN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(800);

#[derive(Clone)]
pub struct BridgeWorkerCore {
    pub audio_in_tx: mpsc::Sender<Vec<i16>>,
    setup_complete: Arc<AtomicBool>,
    cancel: CancellationToken,
    shutdown: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pcm_drops: Arc<AtomicU64>,
}

impl BridgeWorkerCore {
    pub fn open(pcm_drops: Arc<AtomicU64>) -> (Self, mpsc::Receiver<Vec<i16>>) {
        let (audio_in_tx, audio_in_rx) = mpsc::channel(AUDIO_IN_CHANNEL_DEPTH);
        let core = Self {
            audio_in_tx,
            setup_complete: Arc::new(AtomicBool::new(false)),
            cancel: CancellationToken::new(),
            shutdown: Arc::new(Mutex::new(None)),
            pcm_drops,
        };
        (core, audio_in_rx)
    }

    pub fn child_cancel(&self) -> CancellationToken {
        self.cancel.child_token()
    }

    pub fn setup_flag(&self) -> Arc<AtomicBool> {
        self.setup_complete.clone()
    }

    pub async fn attach_worker(&self, join: tokio::task::JoinHandle<()>) {
        let mut guard = self.shutdown.lock().await;
        *guard = Some(join);
    }

    pub async fn wait_until_ready(
        &self,
        provider_label: &str,
        direction: &str,
        timeout_message: &str,
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(20);
        while !self.setup_complete.load(Ordering::SeqCst) {
            if self.cancel.is_cancelled() {
                self.abort().await;
                return Err(anyhow!("{provider_label} setup cancelled for {direction}"));
            }
            if tokio::time::Instant::now() > deadline {
                self.abort().await;
                return Err(anyhow!("{timeout_message}"));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        Ok(())
    }

    pub fn try_send_audio(&self, pcm: Vec<i16>) {
        if pcm.is_empty() || !self.is_ready() {
            return;
        }
        // Drop-on-full is the correct realtime policy (bounded ~5s upload buffer);
        // the failure mode to avoid is *silent* drops — count and log them.
        if self.audio_in_tx.try_send(pcm).is_err() {
            let drops = self.pcm_drops.fetch_add(1, Ordering::Relaxed) + 1;
            if drops == 1 || drops.is_multiple_of(50) {
                tracing::warn!(
                    drops,
                    "bridge audio_in channel full; dropping STT upload frames"
                );
            }
        }
    }

    pub fn is_ready(&self) -> bool {
        self.setup_complete.load(Ordering::SeqCst)
    }

    pub fn ready_flag(&self) -> Arc<AtomicBool> {
        self.setup_complete.clone()
    }

    pub async fn abort(&self) {
        self.cancel.cancel();
        let mut guard = self.shutdown.lock().await;
        if let Some(join) = guard.take() {
            join.abort();
            let _ = join.await;
        }
    }

    /// Graceful stop: cancel so the session can flush final transcripts, then await join.
    pub async fn stop(self) {
        self.cancel.cancel();
        let mut guard = self.shutdown.lock().await;
        let Some(mut join) = guard.take() else {
            return;
        };
        tokio::select! {
            result = &mut join => {
                match result {
                    Ok(()) => {}
                    Err(e) if e.is_cancelled() => {}
                    Err(e) => tracing::warn!("bridge worker join error: {e}"),
                }
            }
            _ = tokio::time::sleep(BRIDGE_STOP_JOIN_TIMEOUT) => {
                tracing::warn!(
                    "bridge worker stop timed out after {}ms — aborting",
                    BRIDGE_STOP_JOIN_TIMEOUT.as_millis()
                );
                join.abort();
                let _ = join.await;
            }
        }
    }
}
