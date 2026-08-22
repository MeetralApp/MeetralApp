//! Outbound TTS delivery for **ElevenLabs** (Speed / Natural).
//!
//! Soniox Provider TTS uses a separate turn-scoped fanout under
//! `voice::providers::soniox::delivery` — do not route Soniox through this module.

mod core;
mod fanout;
mod modes;
mod state;
mod tts;

pub use fanout::spawn_outbound_transcript_fanout;
