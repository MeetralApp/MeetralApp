//! Public ElevenLabs fields exposed on [`ConfigView`] (no API key).

use serde::{Deserialize, Serialize};

use super::elevenlabs_settings::{
    default_elevenlabs_chunk_schedule_preset, default_elevenlabs_crossfade_ms,
    default_elevenlabs_playback_crossfade, default_elevenlabs_similarity_boost,
    default_elevenlabs_speed, default_elevenlabs_stability, default_elevenlabs_tts_language_auto,
    default_elevenlabs_tts_model, default_elevenlabs_tts_synthesis_mode,
    default_elevenlabs_use_speaker_boost, ElevenLabsSettings,
};

/// Wire-facing ElevenLabs settings (flattened into ConfigView). Secrets stay on AppConfig.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElevenLabsPublicSettings {
    pub elevenlabs_voice_id: String,
    #[serde(default)]
    pub elevenlabs_voices: Vec<crate::voice::ElevenLabsVoiceOption>,
    #[serde(default)]
    pub elevenlabs_models: Vec<crate::voice::ElevenLabsModelOption>,
    #[serde(default = "default_elevenlabs_tts_model")]
    pub elevenlabs_tts_model: String,
    #[serde(default = "default_elevenlabs_stability")]
    pub elevenlabs_stability: f32,
    #[serde(default = "default_elevenlabs_similarity_boost")]
    pub elevenlabs_similarity_boost: f32,
    #[serde(default = "default_elevenlabs_speed")]
    pub elevenlabs_speed: f32,
    #[serde(default = "default_elevenlabs_use_speaker_boost")]
    pub elevenlabs_use_speaker_boost: bool,
    #[serde(default = "default_elevenlabs_chunk_schedule_preset")]
    pub elevenlabs_chunk_schedule_preset: crate::voice::config::ElevenLabsChunkSchedulePreset,
    #[serde(default = "default_elevenlabs_tts_language_auto")]
    pub elevenlabs_tts_language_auto: bool,
    #[serde(default)]
    pub elevenlabs_tts_language_code: String,
    #[serde(default = "default_elevenlabs_tts_synthesis_mode")]
    pub elevenlabs_tts_synthesis_mode: crate::voice::config::TtsSynthesisMode,
    #[serde(default = "default_elevenlabs_playback_crossfade")]
    pub elevenlabs_playback_crossfade: bool,
    #[serde(default = "default_elevenlabs_crossfade_ms")]
    pub elevenlabs_crossfade_ms: u32,
    #[serde(default)]
    pub elevenlabs_inbound_voice_id: String,
    #[serde(default = "default_elevenlabs_tts_model")]
    pub elevenlabs_inbound_tts_model: String,
    #[serde(default = "default_elevenlabs_stability")]
    pub elevenlabs_inbound_stability: f32,
    #[serde(default = "default_elevenlabs_similarity_boost")]
    pub elevenlabs_inbound_similarity_boost: f32,
    #[serde(default = "default_elevenlabs_tts_synthesis_mode")]
    pub elevenlabs_inbound_tts_synthesis_mode: crate::voice::config::TtsSynthesisMode,
}

impl From<&ElevenLabsSettings> for ElevenLabsPublicSettings {
    fn from(s: &ElevenLabsSettings) -> Self {
        Self {
            elevenlabs_voice_id: s.elevenlabs_voice_id.clone(),
            elevenlabs_voices: s.elevenlabs_voices.clone(),
            elevenlabs_models: s.elevenlabs_models.clone(),
            elevenlabs_tts_model: s.elevenlabs_tts_model.clone(),
            elevenlabs_stability: s.elevenlabs_stability,
            elevenlabs_similarity_boost: s.elevenlabs_similarity_boost,
            elevenlabs_speed: s.elevenlabs_speed,
            elevenlabs_use_speaker_boost: s.elevenlabs_use_speaker_boost,
            elevenlabs_chunk_schedule_preset: s.elevenlabs_chunk_schedule_preset,
            elevenlabs_tts_language_auto: s.elevenlabs_tts_language_auto,
            elevenlabs_tts_language_code: s.elevenlabs_tts_language_code.clone(),
            elevenlabs_tts_synthesis_mode: s.elevenlabs_tts_synthesis_mode,
            elevenlabs_playback_crossfade: s.elevenlabs_playback_crossfade,
            elevenlabs_crossfade_ms: s.elevenlabs_crossfade_ms,
            elevenlabs_inbound_voice_id: s.elevenlabs_inbound_voice_id.clone(),
            elevenlabs_inbound_tts_model: s.elevenlabs_inbound_tts_model.clone(),
            elevenlabs_inbound_stability: s.elevenlabs_inbound_stability,
            elevenlabs_inbound_similarity_boost: s.elevenlabs_inbound_similarity_boost,
            elevenlabs_inbound_tts_synthesis_mode: s.elevenlabs_inbound_tts_synthesis_mode,
        }
    }
}

impl ElevenLabsPublicSettings {
    /// Merge public view fields into stored settings; keep API key + internal topology flags.
    pub fn merge_into(&self, existing: &ElevenLabsSettings) -> ElevenLabsSettings {
        ElevenLabsSettings {
            elevenlabs_api_key: existing.elevenlabs_api_key.clone(),
            elevenlabs_voice_id: self.elevenlabs_voice_id.clone(),
            elevenlabs_voices: self.elevenlabs_voices.clone(),
            elevenlabs_models: self.elevenlabs_models.clone(),
            elevenlabs_tts_model: self.elevenlabs_tts_model.clone(),
            elevenlabs_stability: self.elevenlabs_stability,
            elevenlabs_similarity_boost: self.elevenlabs_similarity_boost,
            elevenlabs_speed: self.elevenlabs_speed,
            elevenlabs_use_speaker_boost: self.elevenlabs_use_speaker_boost,
            elevenlabs_chunk_schedule_preset: self.elevenlabs_chunk_schedule_preset,
            elevenlabs_tts_language_auto: self.elevenlabs_tts_language_auto,
            elevenlabs_tts_language_code: self.elevenlabs_tts_language_code.clone(),
            elevenlabs_tts_synthesis_mode: self.elevenlabs_tts_synthesis_mode,
            elevenlabs_auto_mode: existing.elevenlabs_auto_mode,
            unified_outbound_topology: existing.unified_outbound_topology,
            elevenlabs_playback_crossfade: self.elevenlabs_playback_crossfade,
            elevenlabs_crossfade_ms: self.elevenlabs_crossfade_ms,
            elevenlabs_inbound_voice_id: self.elevenlabs_inbound_voice_id.clone(),
            elevenlabs_inbound_tts_model: self.elevenlabs_inbound_tts_model.clone(),
            elevenlabs_inbound_stability: self.elevenlabs_inbound_stability,
            elevenlabs_inbound_similarity_boost: self.elevenlabs_inbound_similarity_boost,
            elevenlabs_inbound_tts_synthesis_mode: self.elevenlabs_inbound_tts_synthesis_mode,
        }
    }
}
