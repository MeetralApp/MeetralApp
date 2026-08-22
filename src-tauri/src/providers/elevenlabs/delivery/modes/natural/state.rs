use tokio::time::Instant;

use super::constants::{NATURAL_IDLE_FLUSH, NATURAL_IDLE_MIN_PENDING_CHARS};
use crate::providers::elevenlabs::delivery::core::DeltaTracker;

pub(crate) struct NaturalState {
    pub(crate) committed_bytes: usize,
}

impl NaturalState {
    pub(crate) fn new() -> Self {
        Self { committed_bytes: 0 }
    }

    pub(crate) fn reset(&mut self) {
        self.committed_bytes = 0;
    }

    pub(crate) fn has_pending(&self, tracker: &DeltaTracker) -> bool {
        self.committed_bytes < tracker.last_translated().len()
    }

    fn pending_char_count(&self, tracker: &DeltaTracker) -> usize {
        if !self.has_pending(tracker) {
            return 0;
        }
        tracker.last_translated()[self.committed_bytes..]
            .trim()
            .chars()
            .count()
    }
}

pub(crate) fn idle_deadline(
    state: &NaturalState,
    tracker: &DeltaTracker,
    revision_pending: bool,
) -> Option<Instant> {
    if revision_pending || !state.has_pending(tracker) {
        return None;
    }
    if state.pending_char_count(tracker) < NATURAL_IDLE_MIN_PENDING_CHARS {
        return None;
    }
    Some(Instant::now() + NATURAL_IDLE_FLUSH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::elevenlabs::delivery::core::DeltaTracker;

    #[test]
    fn idle_deadline_requires_min_pending_chars() {
        let mut tracker = DeltaTracker::new();
        let mut natural = NaturalState::new();
        tracker.set_buffer("short");
        assert!(idle_deadline(&natural, &tracker, false).is_none());

        tracker.set_buffer("long enough pending tail");
        assert!(idle_deadline(&natural, &tracker, false).is_some());
        natural.committed_bytes = tracker.last_translated().len();
        assert!(idle_deadline(&natural, &tracker, false).is_none());
    }
}
