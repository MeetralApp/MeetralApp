//! Map fanout text onto xAI utterances (one `text.delta` + `text.done` each).
//!
//! Soniox peels translated prefixes into `AppendDelta` as soon as translate
//! text exists (the UI uses the same events). Waiting for `Flush` /
//! `turn_complete` delays TTS until `SegmentEngine` commits a new segment.
//! Character-count packing is also forbidden (it re-segments translate text).
//!
//! `Flush` is a no-op here: every delta is already an utterance. Gemini /
//! OpenAI unit boundaries stay in their fanout later.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakableUnit {
    pub text: String,
}

#[derive(Debug, Default)]
pub struct UtteranceBuf {}

impl UtteranceBuf {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an xAI utterance as soon as translated text arrives.
    pub fn push_delta(&self, text: &str) -> Vec<SpeakableUnit> {
        if !text.chars().any(|c| !c.is_whitespace()) {
            return Vec::new();
        }
        vec![SpeakableUnit {
            text: text.to_string(),
        }]
    }

    /// Turn end — deltas were already sent; nothing left to speak.
    pub fn flush(&self) -> Vec<SpeakableUnit> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_delta_emits_immediately_without_flush() {
        let b = UtteranceBuf::new();
        let out = b.push_delta("Hello");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "Hello");
        assert!(
            b.flush().is_empty(),
            "Flush must not wait for a new segment and must not re-speak"
        );
    }

    #[test]
    fn each_delta_is_its_own_utterance() {
        let b = UtteranceBuf::new();
        let first = b.push_delta("Hello");
        let second = b.push_delta(" world");
        assert_eq!(first[0].text, "Hello");
        assert_eq!(second[0].text, " world");
    }

    #[test]
    fn whitespace_only_does_not_start_tts() {
        let b = UtteranceBuf::new();
        assert!(b.push_delta("   ").is_empty());
        assert!(b.push_delta("").is_empty());
    }
}
