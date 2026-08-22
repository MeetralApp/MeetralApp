//! Async dual-direction recording session.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use super::encode::{encode_pcm_i16_to_file, RECORD_SAMPLE_RATE};
use super::resample::resample_48k_to_16k;
use crate::config::INPUT_SAMPLE_RATE;
use crate::meeting::store::{
    chunk_path, resolve_recordings_base, wall_ms, AudioChunkRow, MeetingStore,
};

const CHUNK_DURATION_MS: u64 = 30_000;
const CAPTURE_RATE: u32 = INPUT_SAMPLE_RATE; // 48000
const QUEUE_CAP: usize = 256;

pub trait RecordingTap: Send + Sync {
    fn on_pcm(&self, direction: &str, pcm: &[i16]);
}

pub struct DisabledRecordingTap;

impl RecordingTap for DisabledRecordingTap {
    fn on_pcm(&self, _direction: &str, _pcm: &[i16]) {}
}

enum RecMsg {
    Pcm {
        /// `"inbound"` | `"outbound"`
        direction: u8,
        samples: Vec<i16>,
        relative_ms: i64,
    },
    Flush(std::sync::mpsc::SyncSender<()>),
}

const DIR_IN: u8 = 0;
const DIR_OUT: u8 = 1;

fn dir_code(direction: &str) -> u8 {
    if direction == "outbound" {
        DIR_OUT
    } else {
        DIR_IN
    }
}

fn dir_str(code: u8) -> &'static str {
    if code == DIR_OUT {
        "outbound"
    } else {
        "inbound"
    }
}

struct DirectionBuf {
    pcm: Vec<i16>,
    /// Meeting-relative ms at start of current buffer.
    started_at_ms: i64,
    seq: u32,
    /// Total capture samples accepted on this direction (for sample-accurate times).
    samples_accepted: u64,
}

impl DirectionBuf {
    fn new() -> Self {
        Self {
            pcm: Vec::with_capacity((CAPTURE_RATE as usize * 2).max(1024)),
            started_at_ms: 0,
            seq: 0,
            samples_accepted: 0,
        }
    }
}

pub struct RecordingSession {
    tx: mpsc::Sender<RecMsg>,
    /// Wall-clock meeting start (`meeting_record.started_at_ms`).
    meeting_start_ms: AtomicU64,
    closed: AtomicBool,
}

pub type ActiveRecording = Arc<RecordingSession>;
pub type SharedActiveRecording = Arc<Mutex<Option<ActiveRecording>>>;

static FALLBACK_ACTIVE: Mutex<Option<ActiveRecording>> = Mutex::new(None);
static DROPPED_CHUNKS: AtomicU64 = AtomicU64::new(0);

pub fn dropped_recording_chunks() -> u64 {
    DROPPED_CHUNKS.load(Ordering::Relaxed)
}

pub fn set_active_recording(session: Option<ActiveRecording>) {
    if let Ok(mut g) = FALLBACK_ACTIVE.lock() {
        *g = session;
    }
}

pub fn flush_active_recording() {
    let session = FALLBACK_ACTIVE.lock().ok().and_then(|mut g| g.take());
    if let Some(s) = session {
        let handle = std::thread::spawn(move || s.flush_blocking());
        let _ = handle.join();
    }
}

/// Start a recording session for `meeting_id`. Spawns a worker task.
pub fn start_recording_session(
    store: Arc<MeetingStore>,
    meeting_id: String,
    base_dir: PathBuf,
    meeting_start_wall_ms: i64,
) -> Result<ActiveRecording> {
    let (tx, rx) = mpsc::channel::<RecMsg>(QUEUE_CAP);
    let session = Arc::new(RecordingSession {
        tx,
        meeting_start_ms: AtomicU64::new(meeting_start_wall_ms.max(0) as u64),
        closed: AtomicBool::new(false),
    });

    let meeting_id_worker = meeting_id.clone();
    tokio::spawn(async move {
        run_worker(store, meeting_id_worker, base_dir, rx).await;
    });

    Ok(session)
}

impl RecordingSession {
    fn relative_ms_now(&self) -> i64 {
        let start = self.meeting_start_ms.load(Ordering::Relaxed) as i64;
        let now = wall_ms();
        (now - start).max(0)
    }

    pub fn enqueue(&self, direction: &str, pcm: &[i16]) {
        if self.closed.load(Ordering::Relaxed) || pcm.is_empty() {
            return;
        }
        let msg = RecMsg::Pcm {
            direction: dir_code(direction),
            samples: pcm.to_vec(),
            relative_ms: self.relative_ms_now(),
        };
        if self.tx.try_send(msg).is_err() {
            let n = DROPPED_CHUNKS.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::warn!(
                direction,
                dropped_total = n,
                "recording queue full — dropping pcm chunk"
            );
        }
    }

    pub fn flush_blocking(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let (resp_tx, resp_rx) = std::sync::mpsc::sync_channel(1);
        if self.tx.blocking_send(RecMsg::Flush(resp_tx)).is_ok() {
            let _ = resp_rx.recv_timeout(std::time::Duration::from_secs(45));
        }
    }
}

impl RecordingTap for RecordingSession {
    fn on_pcm(&self, direction: &str, pcm: &[i16]) {
        self.enqueue(direction, pcm);
    }
}

pub fn tap_pcm(enabled: bool, direction: &str, pcm: &[i16]) {
    if !enabled || pcm.is_empty() {
        return;
    }
    let session = FALLBACK_ACTIVE.lock().ok().and_then(|g| g.clone());
    if let Some(s) = session {
        s.enqueue(direction, pcm);
    }
}

async fn run_worker(
    store: Arc<MeetingStore>,
    meeting_id: String,
    base_dir: PathBuf,
    mut rx: mpsc::Receiver<RecMsg>,
) {
    let mut inbound = DirectionBuf::new();
    let mut outbound = DirectionBuf::new();
    let samples_per_chunk = (CAPTURE_RATE as u64 * CHUNK_DURATION_MS / 1000) as usize;

    while let Some(msg) = rx.recv().await {
        match msg {
            RecMsg::Pcm {
                direction,
                samples,
                relative_ms,
            } => {
                let buf = if direction == DIR_OUT {
                    &mut outbound
                } else {
                    &mut inbound
                };
                if buf.pcm.is_empty() {
                    // Prefer wall-relative stamp; also keep sample counter for duration.
                    buf.started_at_ms = relative_ms;
                }
                buf.pcm.extend_from_slice(&samples);
                buf.samples_accepted += samples.len() as u64;
                while buf.pcm.len() >= samples_per_chunk {
                    let chunk: Vec<i16> = buf.pcm.drain(..samples_per_chunk).collect();
                    let started = buf.started_at_ms;
                    let seq = buf.seq;
                    buf.seq += 1;
                    let chunk_ms = (chunk.len() as u64 * 1000 / CAPTURE_RATE as u64) as i64;
                    buf.started_at_ms = started + chunk_ms;
                    persist_chunk_blocking(
                        store.clone(),
                        base_dir.clone(),
                        meeting_id.clone(),
                        dir_str(direction),
                        seq,
                        started,
                        chunk,
                    )
                    .await;
                }
            }
            RecMsg::Flush(done) => {
                for (code, buf) in [(DIR_IN, &mut inbound), (DIR_OUT, &mut outbound)] {
                    if buf.pcm.is_empty() {
                        continue;
                    }
                    let chunk = std::mem::take(&mut buf.pcm);
                    let started = buf.started_at_ms;
                    let seq = buf.seq;
                    buf.seq += 1;
                    persist_chunk_blocking(
                        store.clone(),
                        base_dir.clone(),
                        meeting_id.clone(),
                        dir_str(code),
                        seq,
                        started,
                        chunk,
                    )
                    .await;
                }
                let _ = done.send(());
                break;
            }
        }
    }
}

async fn persist_chunk_blocking(
    store: Arc<MeetingStore>,
    base_dir: PathBuf,
    meeting_id: String,
    direction: &'static str,
    seq: u32,
    started_at_ms: i64,
    pcm_48k: Vec<i16>,
) {
    let result = tokio::task::spawn_blocking(move || {
        persist_chunk(
            &store,
            &base_dir,
            &meeting_id,
            direction,
            seq,
            started_at_ms,
            &pcm_48k,
        )
    })
    .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::error!(error = %e, direction, seq, "persist audio chunk failed");
        }
        Err(e) => {
            tracing::error!(error = %e, direction, seq, "persist audio chunk join failed");
        }
    }
}

fn persist_chunk(
    store: &MeetingStore,
    base_dir: &std::path::Path,
    meeting_id: &str,
    direction: &str,
    seq: u32,
    started_at_ms: i64,
    pcm_48k: &[i16],
) -> Result<()> {
    let pcm_16k = resample_48k_to_16k(pcm_48k).context("resample 48k→16k")?;
    if pcm_16k.is_empty() {
        return Ok(());
    }
    let duration_ms = (pcm_16k.len() as u64 * 1000 / RECORD_SAMPLE_RATE as u64) as i64;
    let path = chunk_path(base_dir, meeting_id, direction, seq);
    encode_pcm_i16_to_file(&pcm_16k, &path)?;
    // Prefer display path without Windows `\\?\` prefix when possible.
    let abs = std::fs::canonicalize(&path)
        .unwrap_or_else(|_| path.clone())
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    let byte_size = std::fs::metadata(&path)
        .map(|m| i64::try_from(m.len()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    store.insert_audio_chunk(AudioChunkRow {
        id: uuid::Uuid::new_v4().to_string(),
        meeting_id: meeting_id.to_string(),
        direction: direction.to_string(),
        sequence: seq as i64,
        file_path: abs,
        duration_ms,
        sample_rate: RECORD_SAMPLE_RATE as i64,
        started_at_ms,
        byte_size,
    })?;
    Ok(())
}

/// Helper for meeting start: resolve folder and start session if enabled.
pub fn maybe_start_for_meeting(
    store: Arc<MeetingStore>,
    app: &tauri::AppHandle,
    meeting_id: &str,
    record_enabled: bool,
    save_folder: &str,
) -> Result<()> {
    set_active_recording(None);
    if !record_enabled {
        return Ok(());
    }
    let base = resolve_recordings_base(app, save_folder).context("resolve recordings base")?;
    std::fs::create_dir_all(&base).ok();
    let meeting_start_ms = store
        .get_meeting(meeting_id, false)
        .map(|m| m.started_at_ms)
        .unwrap_or_else(|_| wall_ms());
    let session = start_recording_session(store, meeting_id.to_string(), base, meeting_start_ms)?;
    set_active_recording(Some(session));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_tap_noop() {
        DisabledRecordingTap.on_pcm("inbound", &[1, 2, 3]);
    }

    #[test]
    fn dir_roundtrip() {
        assert_eq!(dir_str(dir_code("outbound")), "outbound");
        assert_eq!(dir_str(dir_code("inbound")), "inbound");
    }
}
