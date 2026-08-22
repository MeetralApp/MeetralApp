use std::time::Duration;

/// If no sentence terminator appears but the uncommitted buffer grows past this
/// many characters, commit at the last word boundary so added latency stays
/// bounded for long, comma-only run-on clauses.
pub const SENTENCE_MAX_CHARS: usize = 220;

/// Commit the pending tail when no new interim arrives for this long.
pub const NATURAL_IDLE_FLUSH: Duration = Duration::from_millis(1200);

/// Do not arm the idle timer until at least this many pending chars.
pub const NATURAL_IDLE_MIN_PENDING_CHARS: usize = 20;
