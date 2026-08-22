pub mod config;
pub mod shared;

// Back-compat module aliases: preserve `crate::voice::elevenlabs` and
// `crate::voice::types` paths used outside `voice/` after the move into
// `providers/` and `shared/`.
pub use crate::providers::elevenlabs;
pub use shared::debug;
pub use shared::types;

pub use crate::providers::elevenlabs::worker::{
    spawn_elevenlabs_tts_worker, ElevenLabsWorkerConfig,
};
pub use crate::providers::elevenlabs::{
    list_models as list_elevenlabs_models, list_voices as list_elevenlabs_voices,
    preview_voice as preview_elevenlabs_voice, validate_voice as validate_elevenlabs_voice,
    ElevenLabsModelOption, ElevenLabsVoiceOption,
};
pub use crate::providers::soniox::list_stt_models as list_soniox_stt_models;
pub use crate::providers::soniox::tts::worker::{spawn_soniox_tts_worker, SonioxTtsWorkerConfig};
pub use crate::providers::soniox::tts::{
    list_tts_models as list_soniox_tts_models, list_voices as list_soniox_voices_for_model,
    preview_voice as preview_soniox_voice, spawn_soniox_transcript_fanout, SonioxTtsModelOption,
    SonioxVoiceOption,
};
pub use config::{
    default_elevenlabs_chunk_schedule_preset, default_elevenlabs_similarity_boost,
    default_elevenlabs_speed, default_elevenlabs_stability, default_elevenlabs_tts_language_auto,
    default_elevenlabs_tts_model, default_elevenlabs_tts_synthesis_mode,
    default_elevenlabs_use_speaker_boost, ElevenLabsChunkSchedulePreset, TtsSynthesisMode,
    ELEVENLABS_KEYCHAIN_ACCOUNT,
};

/// List Soniox TTS voices for the default (or unspecified) model.
pub async fn list_soniox_voices(api_key: &str) -> Result<Vec<SonioxVoiceOption>, String> {
    list_soniox_voices_for_model(api_key, None).await
}
pub use crate::providers::elevenlabs::spawn_outbound_transcript_fanout;
pub use shared::tts_command::TtsTextCommand;
pub use shared::types::{VoiceCloneLatencyEvent, VoiceTtsStatus, VoiceTtsStatusPayload};
