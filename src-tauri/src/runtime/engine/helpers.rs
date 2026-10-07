use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

pub const BRIDGE_FATAL_USER_MESSAGE: &str = "Translation connection lost. Stop and Start again.";

pub const AUDIO_HEARTBEAT_STALE_MS: u64 = 3000;
pub const AUDIO_STARTUP_GRACE_MS: u64 = 8000;
pub const AUDIO_RECOVERY_CONFIRM_MS: u64 = 2000;
pub const MAX_AUDIO_RECONNECT_ATTEMPTS: u32 = 5;
pub const AUDIO_DIRECT_LOST_MESSAGE: &str = "Audio device lost. Direct relay stopped.";
pub const AUDIO_OUTBOUND_LOST_MESSAGE: &str =
    "Audio device lost (microphone). Translation stopped.";
pub const AUDIO_INBOUND_LOST_MESSAGE: &str =
    "Audio device lost (meeting capture). Translation stopped.";
pub const PROACTIVE_SESSION_MS: i64 = 60 * 60 * 1000;
/// Default idle duration when auto-end is on (5 minutes). Runtime reads
/// `AppConfig::auto_end_meeting_idle_ms` — this constant is the On default.
pub const MEETING_IDLE_NO_SEGMENT_MS: u64 =
    crate::config::DEFAULT_AUTO_END_MEETING_AFTER_MIN as u64 * 60_000;
pub const ENSURE_DIRECT_AUDIO_EVERY_TICKS: u32 = 5;
pub const WATCHDOG_TICK_STABLE_MS: u64 = 2000;
pub const WATCHDOG_TICK_IDLE_MS: u64 = 8000;
pub const WATCHDOG_DEVICE_SCAN_EVERY_TICKS_STABLE: u32 = 1;
/// Idle ticks also scan every tick: OS notifications are the fast path, this is
/// the fallback ceiling for re-plug detection (was 4 ticks / ~32 s).
pub const WATCHDOG_DEVICE_SCAN_EVERY_TICKS_IDLE: u32 = 1;
/// Time the device catalog must stay unchanged before ensure/recovery may act
/// on it. Opening a Bluetooth HFP mic or restarting a virtual driver's audio
/// engine churns the endpoint list for a few seconds; acting on a mid-churn
/// enumeration is what turned transient gaps into user-visible disconnects.
pub const AUDIO_CATALOG_STABLE_MS: u64 = 3000;

pub fn watchdog_tick_sleep_ms(degraded: bool) -> u64 {
    if degraded {
        WATCHDOG_TICK_STABLE_MS
    } else {
        WATCHDOG_TICK_IDLE_MS
    }
}

pub fn watchdog_should_scan_devices(degraded: bool, tick_count: u32) -> bool {
    let every = if degraded {
        WATCHDOG_DEVICE_SCAN_EVERY_TICKS_STABLE
    } else {
        WATCHDOG_DEVICE_SCAN_EVERY_TICKS_IDLE
    };
    tick_count.is_multiple_of(every)
}

/// Monotonic milliseconds for staleness/backoff/retry scheduling. Immune to
/// wall-clock jumps; must share the same base as `CaptureHeartbeat` timestamps.
pub fn unix_ms_now_u64() -> u64 {
    crate::audio::monotonic_ms()
}

pub fn audio_backoff_ms(attempt: u32) -> u64 {
    const BACKOFF: [u64; 5] = [1000, 2000, 4000, 8000, 8000];
    BACKOFF
        .get(attempt.saturating_sub(1) as usize)
        .copied()
        .unwrap_or(8000)
}

pub fn unix_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn format_connection_gap_stamp() -> String {
    let total_secs = unix_ms_now().max(0) / 1000;
    let hours = (total_secs / 3600) % 24;
    let minutes = (total_secs / 60) % 60;
    format!("{hours:02}:{minutes:02}")
}

/// Read the in-memory config snapshot managed by the app without touching disk.
/// This is the single source of truth for config; the watchdog and recovery
/// paths must not re-read `config.json` from disk while holding the engine lock.
pub async fn config_from_state(app: &AppHandle) -> AppConfig {
    match app.try_state::<Mutex<AppConfig>>() {
        Some(state) => state.lock().await.clone(),
        None => AppConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::audio_backoff_ms;

    #[test]
    fn audio_backoff_ms_table() {
        assert_eq!(audio_backoff_ms(0), 1000);
        assert_eq!(audio_backoff_ms(1), 1000);
        assert_eq!(audio_backoff_ms(2), 2000);
        assert_eq!(audio_backoff_ms(3), 4000);
        assert_eq!(audio_backoff_ms(4), 8000);
        assert_eq!(audio_backoff_ms(5), 8000);
        assert_eq!(audio_backoff_ms(99), 8000);
    }
}
