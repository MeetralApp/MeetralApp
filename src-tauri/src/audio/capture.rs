use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, OnceLock,
};
use std::time::Instant;

use tokio::sync::mpsc;

/// Process-wide monotonic clock base. All elapsed/staleness/backoff timing is
/// measured relative to this base so it is immune to wall-clock jumps (NTP sync,
/// manual clock changes, DST). Wall-clock time is reserved for UI display only.
fn monotonic_base() -> Instant {
    static BASE: OnceLock<Instant> = OnceLock::new();
    *BASE.get_or_init(Instant::now)
}

/// Milliseconds elapsed since the process monotonic base.
pub fn monotonic_ms() -> u64 {
    monotonic_base().elapsed().as_millis() as u64
}

pub const CAPTURE_CHANNEL_DEPTH: usize = 16;

pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    device_id: String,
    heartbeat: CaptureHeartbeat,
}

impl CaptureHandle {
    pub(crate) fn new(
        stop: Arc<AtomicBool>,
        thread: std::thread::JoinHandle<()>,
        device_id: String,
        heartbeat: CaptureHeartbeat,
    ) -> Self {
        Self {
            stop,
            thread: Some(thread),
            device_id,
            heartbeat,
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn heartbeat(&self) -> &CaptureHeartbeat {
        &self.heartbeat
    }
}

#[derive(Clone, Default)]
pub struct CaptureHeartbeat {
    last_frame_ms: Arc<AtomicU64>,
}

impl CaptureHeartbeat {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn touch(&self) {
        self.last_frame_ms
            .store(monotonic_ms().max(1), Ordering::Relaxed);
    }

    pub fn last_frame_ms(&self) -> u64 {
        self.last_frame_ms.load(Ordering::Relaxed)
    }

    pub fn is_stale(
        &self,
        now_ms: u64,
        threshold_ms: u64,
        started_at_ms: Option<u64>,
        startup_grace_ms: u64,
    ) -> bool {
        if let Some(started) = started_at_ms {
            if now_ms.saturating_sub(started) < startup_grace_ms {
                return false;
            }
        }
        let last = self.last_frame_ms.load(Ordering::Relaxed);
        last == 0 || now_ms.saturating_sub(last) > threshold_ms
    }
}

pub type CaptureSender = mpsc::Sender<Vec<i16>>;

#[derive(Debug, Clone)]
pub struct AudioFaultEvent {
    pub device_id: String,
    pub device_name: String,
    pub reason: String,
}

pub type AudioFaultSender = mpsc::Sender<AudioFaultEvent>;

#[cfg(windows)]
pub use super::backend::windows::capture::{
    start_meeting_capture_for_config, start_user_mic_capture,
};

#[cfg(target_os = "macos")]
pub use super::backend::macos::capture::{
    start_meeting_capture_for_config, start_user_mic_capture,
};

#[cfg(not(any(windows, target_os = "macos")))]
pub fn start_user_mic_capture(
    _config: &crate::config::AppConfig,
    _devices: &[super::device::AudioDeviceInfo],
    _tx: CaptureSender,
    _heartbeat: CaptureHeartbeat,
    _fault_tx: Option<AudioFaultSender>,
) -> anyhow::Result<CaptureHandle> {
    anyhow::bail!("Capture is not supported on this platform")
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn start_meeting_capture_for_config(
    _config: &crate::config::AppConfig,
    _devices: &[super::device::AudioDeviceInfo],
    _tx: CaptureSender,
    _heartbeat: CaptureHeartbeat,
    _fault_tx: Option<AudioFaultSender>,
) -> anyhow::Result<CaptureHandle> {
    anyhow::bail!("Meeting capture is not supported on this platform")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_stale_when_never_touched_after_grace() {
        let hb = CaptureHeartbeat::new();
        let now = 1_000_000u64;
        assert!(!hb.is_stale(now, 1000, Some(now), 5000));
        assert!(hb.is_stale(now, 1000, Some(now - 10_000), 1000));
    }

    #[test]
    fn heartbeat_fresh_after_touch() {
        let hb = CaptureHeartbeat::new();
        hb.touch();
        let last = hb.last_frame_ms();
        assert!(last >= 1);
        assert!(!hb.is_stale(last + 100, 3000, None, 0));
    }
}
