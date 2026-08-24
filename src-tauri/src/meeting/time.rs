//! Meeting timeline clocks.
//!
//! Convention (SSOT):
//! - `meeting_record.started_at_ms` / `ended_at_ms` — **wall** (Unix ms) for library + header timer.
//! - `transcript_segment.started_at_ms` / `ended_at_ms` — **meeting-relative** ms (0 = meeting start).
//! - Live writes use a process **mono anchor** derived from wall elapsed so relative stays
//!   continuous across app restart while still immune to NTP jitter during the session.

use crate::audio::monotonic_ms;

pub(crate) fn wall_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Monotonic process time that corresponds to meeting wall-start.
///
/// `mono_anchor = mono_now − max(0, wall_now − meeting_wall_started)`
/// so `mono_now − mono_anchor ≈ wall_now − meeting_wall_started`.
pub fn mono_anchor_for_meeting(
    meeting_wall_started_ms: i64,
    wall_now_ms: i64,
    mono_now_ms: i64,
) -> i64 {
    let wall_elapsed = (wall_now_ms - meeting_wall_started_ms).max(0);
    (mono_now_ms - wall_elapsed).max(0)
}

/// Capture mono anchor from current wall + mono clocks.
pub fn capture_mono_anchor(meeting_wall_started_ms: i64) -> i64 {
    mono_anchor_for_meeting(
        meeting_wall_started_ms,
        wall_now_ms(),
        monotonic_ms() as i64,
    )
}

/// Elapsed ms since meeting start from a mono timestamp and anchor.
pub fn relative_ms(mono_ms: i64, mono_anchor_ms: i64) -> i64 {
    (mono_ms - mono_anchor_ms).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_aligns_mono_elapsed_with_wall_elapsed() {
        // Meeting started 90s ago on wall; process mono is at 200s.
        let meeting_wall = 1_000_000;
        let wall_now = meeting_wall + 90_000;
        let mono_now = 200_000;
        let anchor = mono_anchor_for_meeting(meeting_wall, wall_now, mono_now);
        assert_eq!(anchor, 110_000);
        assert_eq!(relative_ms(mono_now, anchor), 90_000);
    }

    #[test]
    fn relative_never_negative() {
        assert_eq!(relative_ms(50, 100), 0);
    }

    #[test]
    fn wall_elapsed_clamped_when_clock_skewed() {
        let meeting_wall = 1_000_000;
        let wall_now = meeting_wall - 5_000; // clock jumped back
        let mono_now = 50_000;
        let anchor = mono_anchor_for_meeting(meeting_wall, wall_now, mono_now);
        assert_eq!(anchor, mono_now);
        assert_eq!(relative_ms(mono_now, anchor), 0);
    }
}
