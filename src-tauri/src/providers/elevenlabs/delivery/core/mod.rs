//! Text processing shared by Speed and Natural delivery modes.
//!
//! Sentence/merge helpers live in `voice::shared::tts_text`; this module keeps
//! the ElevenLabs `DeltaTracker` and re-exports the shared API for local use.

mod tracker;

pub(crate) use crate::voice::shared::tts_text::{
    is_junk_tts_fragment, last_committable_sentence_end, last_word_boundary, merge_streaming_text,
    normalize_interim_translated, streaming_text_ends_sentence,
};
pub(crate) use tracker::DeltaTracker;
