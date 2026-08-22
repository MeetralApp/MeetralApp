//! Shared streaming-text helpers for Natural-style TTS delivery.
//!
//! Sentence boundary detection and interim merge live here so ElevenLabs and
//! Soniox delivery can share one implementation. Provider-specific fanout /
//! commit loops stay under `providers/<vendor>/`.

mod constants;
mod sentence;
mod text;

pub use constants::{NATURAL_IDLE_FLUSH, NATURAL_IDLE_MIN_PENDING_CHARS, SENTENCE_MAX_CHARS};
pub use sentence::{
    last_committable_sentence_end, last_word_boundary, streaming_text_ends_sentence,
};
pub use text::{is_junk_tts_fragment, merge_streaming_text, normalize_interim_translated};
