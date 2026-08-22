use tokio::time::Instant;

use super::constants::{
    REVISION_DEBOUNCE, REVISION_MAX_WAIT, SPEED_SENTENCE_FLUSH_DEBOUNCE, SPEED_SENTENCE_MIN_CHARS,
};
use crate::providers::elevenlabs::delivery::core::{last_committable_sentence_end, DeltaTracker};

pub(crate) struct SpeedState {
    pub(crate) el_sent_anchor: String,
    pub(crate) has_unflushed_text: bool,
    pub(crate) chars_since_last_flush: usize,
    pending_revision: Option<String>,
    revision_first_seen_at: Option<Instant>,
    pub(crate) revisions_debounced: u32,
    pub(crate) major_rewrites: u32,
    pub(crate) last_sentence_flush_byte: usize,
    pending_sentence_end: Option<usize>,
}

impl SpeedState {
    pub(crate) fn new() -> Self {
        Self {
            el_sent_anchor: String::new(),
            has_unflushed_text: false,
            chars_since_last_flush: 0,
            pending_revision: None,
            revision_first_seen_at: None,
            revisions_debounced: 0,
            major_rewrites: 0,
            last_sentence_flush_byte: 0,
            pending_sentence_end: None,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.el_sent_anchor.clear();
        self.has_unflushed_text = false;
        self.chars_since_last_flush = 0;
        self.pending_revision = None;
        self.revision_first_seen_at = None;
        self.last_sentence_flush_byte = 0;
        self.pending_sentence_end = None;
    }

    pub(crate) fn revision_pending(&self) -> bool {
        self.pending_revision.is_some()
    }

    pub(crate) fn revision_deadline(&self) -> Option<Instant> {
        self.pending_revision.as_ref()?;
        let debounce_at = Instant::now() + REVISION_DEBOUNCE;
        let Some(first) = self.revision_first_seen_at else {
            return Some(debounce_at);
        };
        let max_at = first + REVISION_MAX_WAIT;
        let now = Instant::now();
        if now >= max_at {
            return Some(now);
        }
        Some(if max_at < debounce_at {
            max_at
        } else {
            debounce_at
        })
    }

    pub(crate) fn arm_revision_debounce(&mut self, translated: String) {
        if self.revision_first_seen_at.is_none() {
            self.revision_first_seen_at = Some(Instant::now());
            self.revisions_debounced += 1;
        }
        self.pending_revision = Some(translated);
    }

    pub(crate) fn take_pending_revision(&mut self) -> Option<String> {
        self.revision_first_seen_at = None;
        self.pending_revision.take()
    }

    /// Arm a debounced sentence-boundary flush when a new complete sentence is detected.
    pub(crate) fn arm_sentence_flush(&mut self, tracker: &DeltaTracker) -> Option<Instant> {
        self.pending_sentence_end = None;
        if !self.has_unflushed_text {
            return None;
        }
        let text = tracker.last_translated();
        let end = last_committable_sentence_end(text)?;
        if end <= self.last_sentence_flush_byte {
            return None;
        }
        let chars = text[self.last_sentence_flush_byte..end].chars().count();
        if chars < SPEED_SENTENCE_MIN_CHARS {
            return None;
        }
        self.pending_sentence_end = Some(end);
        Some(Instant::now() + SPEED_SENTENCE_FLUSH_DEBOUNCE)
    }

    pub(crate) fn take_pending_sentence_end(&mut self) -> Option<usize> {
        self.pending_sentence_end.take()
    }

    pub(crate) fn clear_pending_sentence_end(&mut self) {
        self.pending_sentence_end = None;
    }
}
