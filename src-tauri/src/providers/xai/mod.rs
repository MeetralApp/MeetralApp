//! xAI custom voice TTS — not a live `AiProvider`.

pub mod api;
pub mod config;
pub mod protocol;
pub mod worker;

pub use api::{list_voices, preview_voice, test_api_key, validate_voice, XaiVoiceOption};
pub use worker::{spawn_xai_tts_worker, XaiWorkerConfig};
