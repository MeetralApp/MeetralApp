pub(crate) struct DeltaTracker {
    last_translated: String,
}

impl DeltaTracker {
    pub(crate) fn new() -> Self {
        Self {
            last_translated: String::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.last_translated.clear();
    }

    pub(crate) fn reanchor(&mut self, anchor: &str) {
        self.last_translated = anchor.to_string();
    }

    pub(crate) fn set_buffer(&mut self, text: &str) {
        self.last_translated = text.to_string();
    }

    pub(crate) fn last_translated(&self) -> &str {
        &self.last_translated
    }

    pub(crate) fn append_translated(&mut self, translated: &str) -> Option<(String, bool)> {
        if translated.is_empty() {
            return None;
        }
        if translated == self.last_translated {
            return None;
        }
        if translated.starts_with(&self.last_translated) {
            let delta = translated[self.last_translated.len()..].to_string();
            self.last_translated = translated.to_string();
            if delta.is_empty() {
                return None;
            }
            return Some((delta, false));
        }
        let delta = translated.to_string();
        self.last_translated = translated.to_string();
        Some((delta, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::shared::tts_text::merge_streaming_text;

    #[test]
    fn delta_tracker_emits_suffix_only() {
        let mut tracker = DeltaTracker::new();
        assert_eq!(
            tracker.append_translated("Hello"),
            Some(("Hello".to_string(), false))
        );
        assert_eq!(
            tracker.append_translated("Hello world"),
            Some((" world".to_string(), false))
        );
        assert_eq!(tracker.append_translated("Hello world"), None);
    }

    #[test]
    fn delta_tracker_non_prefix_requests_flush() {
        let mut tracker = DeltaTracker::new();
        tracker.append_translated("Hello");
        assert_eq!(
            tracker.append_translated("Hi there"),
            Some(("Hi there".to_string(), true))
        );
    }

    #[test]
    fn reanchor_prevents_duplicate_after_midturn_flush() {
        let mut tracker = DeltaTracker::new();
        assert_eq!(
            tracker.append_translated("Xin chào"),
            Some(("Xin chào".to_string(), false))
        );
        tracker.reanchor("Xin chào");
        assert_eq!(
            tracker.append_translated("Xin chào mọi người"),
            Some((" mọi người".to_string(), false))
        );
    }

    #[test]
    fn fragment_stream_never_requests_flush_after_merge() {
        let mut tracker = DeltaTracker::new();
        let fragments = ["Xin chào", "mọi", "người", "tôi", "xin giới thiệu"];
        for frag in fragments {
            let merged = merge_streaming_text(tracker.last_translated(), frag);
            if let Some((_, needs_flush)) = tracker.append_translated(&merged) {
                assert!(
                    !needs_flush,
                    "fragment {frag:?} must not trigger a revision flush"
                );
            }
        }
        assert_eq!(
            tracker.last_translated(),
            "Xin chào mọi người tôi xin giới thiệu"
        );
    }

    #[test]
    fn reset_re_sends_everything_after_clear() {
        let mut tracker = DeltaTracker::new();
        tracker.append_translated("Xin chào");
        tracker.reset();
        assert_eq!(
            tracker.append_translated("Hoàn toàn khác"),
            Some(("Hoàn toàn khác".to_string(), false))
        );
    }
}
