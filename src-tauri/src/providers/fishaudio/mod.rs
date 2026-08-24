//! Fish Audio custom voice TTS — not a live `AiProvider`.

pub mod api;
pub mod config;
pub mod protocol;
pub mod worker;

pub use api::{
    list_models, list_voices, preview_voice, test_api_key, validate_voice, FishAudioModelOption,
    FishAudioVoiceOption,
};
pub use worker::{spawn_fishaudio_tts_worker, FishAudioWorkerConfig};
