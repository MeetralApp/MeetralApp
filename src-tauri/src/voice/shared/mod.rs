//! Provider-agnostic TTS infrastructure shared across voice providers.
//!
//! Types, latency tracking, the generic TTS text-command enum, shared text
//! helpers, and debug helpers. Provider-specific delivery lives under each
//! provider:
//! - ElevenLabs Speed/Natural: `providers::elevenlabs::delivery`
//! - Soniox stream fanout: `providers::soniox::tts::delivery`

pub mod debug;
pub mod latency;
pub mod tts_command;
pub mod tts_text;
pub mod types;
