//! Soniox-specific transcript → TTS delivery.
//!
//! Turn-scoped prefix deltas + one Flush per utterance for both Clone and
//! Provider TTS. Soniox `text_end` closes the stream on Flush.

mod fanout;
mod stream;

pub use fanout::spawn_soniox_transcript_fanout;
