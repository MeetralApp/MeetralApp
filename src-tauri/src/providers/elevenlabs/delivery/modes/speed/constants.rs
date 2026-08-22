use std::time::Duration;

/// OpenAI safety net when `turn_complete` is delayed (paired_done debounce is separate).
pub(crate) const OPENAI_IDLE_FLUSH: Duration = Duration::from_millis(300);

/// Short interjections flushed early on Gemini segment boundaries (fast lane).
pub(crate) const FAST_LANE_MAX_CHARS: usize = 48;

/// Confirm a finished short segment is a standalone interjection (not the head of a
/// longer turn) before hard-flushing. A continuation delta or `turn_complete` cancels it.
pub(crate) const FAST_LANE_DEBOUNCE: Duration = Duration::from_millis(250);

pub(crate) const REVISION_DEBOUNCE: Duration = Duration::from_millis(200);
pub(crate) const REVISION_MAX_WAIT: Duration = Duration::from_millis(800);
pub(crate) const MAJOR_REWRITE_MIN_LCP: usize = 4;

/// Speed mode: debounce before flushing at a detected sentence boundary.
pub(crate) const SPEED_SENTENCE_FLUSH_DEBOUNCE: Duration = Duration::from_millis(200);

/// Speed mode: minimum chars in the new sentence span before arming a boundary flush.
pub(crate) const SPEED_SENTENCE_MIN_CHARS: usize = 12;

/// Short interjections without `.?!` may still use segment fast-lane below this length.
pub(crate) const FAST_LANE_INTERJECTION_MAX_CHARS: usize = 12;
