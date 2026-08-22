use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[cfg(not(any(windows, target_os = "macos")))]
#[cfg(not(any(windows, target_os = "macos")))]
use super::playback_buffer::PlaybackBufferConfig;

pub struct PlaybackHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl PlaybackHandle {
    pub(crate) fn new(stop: Arc<AtomicBool>, thread: std::thread::JoinHandle<()>) -> Self {
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for PlaybackHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(windows)]
pub use super::backend::windows::playback::{start_playback, start_playback_for_role};

#[cfg(target_os = "macos")]
pub use super::backend::macos::playback::{start_playback, start_playback_for_role};

#[cfg(not(any(windows, target_os = "macos")))]
pub fn start_playback(
    _resolved: super::device::ResolvedDevice,
    _rx: tokio::sync::mpsc::Receiver<Vec<i16>>,
    _buffer_config: PlaybackBufferConfig,
) -> anyhow::Result<PlaybackHandle> {
    anyhow::bail!("Playback is not supported on this platform")
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn start_playback_for_role(
    _role: super::device::AudioRole,
    _config: &crate::config::AppConfig,
    _devices: &[super::device::AudioDeviceInfo],
    _rx: tokio::sync::mpsc::Receiver<Vec<i16>>,
) -> anyhow::Result<PlaybackHandle> {
    anyhow::bail!("Playback is not supported on this platform")
}
