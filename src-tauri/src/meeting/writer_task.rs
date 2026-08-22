//! Async queue for transcript DB writes — keeps SQLite off the emit hot path.

use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};

use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::meeting::transcript_writer::SegmentPersistJob;
use crate::meeting::{MeetingStore, SharedTranscriptWriter};

pub const TRANSCRIPT_DB_QUEUE_DEPTH: usize = 256;

#[derive(Default)]
pub struct TranscriptDbMetrics {
    pub queue_depth: AtomicUsize,
    pub dropped_events: AtomicU64,
}

pub type SharedTranscriptDbMetrics = Arc<TranscriptDbMetrics>;

pub fn new_transcript_db_metrics() -> SharedTranscriptDbMetrics {
    Arc::new(TranscriptDbMetrics::default())
}

pub struct TranscriptDbWriterHandle {
    tx: mpsc::Sender<SegmentPersistJob>,
    metrics: SharedTranscriptDbMetrics,
}

impl TranscriptDbWriterHandle {
    pub fn enqueue(&self, job: SegmentPersistJob) {
        match self.tx.try_send(job) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(job)) => {
                self.metrics.dropped_events.fetch_add(1, Ordering::Relaxed);
                tracing::error!(
                    "transcript DB queue full (depth {TRANSCRIPT_DB_QUEUE_DEPTH}); dropping direction={}",
                    job.direction
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::warn!("transcript DB writer closed");
            }
        }
    }

    pub fn metrics(&self) -> &SharedTranscriptDbMetrics {
        &self.metrics
    }
}

impl Clone for TranscriptDbWriterHandle {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

pub fn spawn_transcript_db_writer(
    store: Arc<MeetingStore>,
    writer: Arc<SharedTranscriptWriter>,
    app: AppHandle,
    metrics: SharedTranscriptDbMetrics,
) -> (TranscriptDbWriterHandle, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel(TRANSCRIPT_DB_QUEUE_DEPTH);
    let enqueue_tx = tx.clone();
    let handle = TranscriptDbWriterHandle {
        tx: enqueue_tx,
        metrics: metrics.clone(),
    };
    let task = tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            metrics.queue_depth.store(rx.len(), Ordering::Relaxed);
            // rusqlite is synchronous — keep it off the async runtime worker, but
            // await each job before the next recv so segment order is preserved.
            let writer = writer.clone();
            let store = store.clone();
            let app = app.clone();
            if let Err(e) =
                tokio::task::spawn_blocking(move || writer.persist(&job, &store, &app)).await
            {
                tracing::error!("transcript persist task failed to join: {e}");
            }
        }
    });
    (handle, task)
}
