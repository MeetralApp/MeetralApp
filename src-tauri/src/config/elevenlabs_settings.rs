use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevenLabsSettings {
    #[serde(default)]
    pub elevenlabs_api_key: String,
    #[serde(default)]
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
    #[serde(default = "default_elevenlabs_auto_mode")]
    pub elevenlabs_auto_mode: bool,
    #[serde(default = "default_unified_outbound_topology")]
    pub unified_outbound_topology: bool,
    /// Meeting → You ElevenLabs voice (shared API key with outbound).
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

pub(crate) fn default_unified_outbound_topology() -> bool {
    true
}

pub(crate) fn default_elevenlabs_tts_model() -> String {
    crate::voice::default_elevenlabs_tts_model()
}

pub(crate) fn default_elevenlabs_stability() -> f32 {
    crate::voice::default_elevenlabs_stability()
}

pub(crate) fn default_elevenlabs_similarity_boost() -> f32 {
    crate::voice::default_elevenlabs_similarity_boost()
}

pub(crate) fn default_elevenlabs_speed() -> f32 {
    crate::voice::default_elevenlabs_speed()
}

pub(crate) fn default_elevenlabs_use_speaker_boost() -> bool {
    crate::voice::default_elevenlabs_use_speaker_boost()
}

pub(crate) fn default_elevenlabs_chunk_schedule_preset(
) -> crate::voice::config::ElevenLabsChunkSchedulePreset {
    crate::voice::default_elevenlabs_chunk_schedule_preset()
}

pub(crate) fn default_elevenlabs_tts_language_auto() -> bool {
    crate::voice::default_elevenlabs_tts_language_auto()
}

pub(crate) fn default_elevenlabs_tts_synthesis_mode() -> crate::voice::config::TtsSynthesisMode {
    crate::voice::default_elevenlabs_tts_synthesis_mode()
}

pub(crate) fn default_elevenlabs_auto_mode() -> bool {
    crate::voice::config::default_elevenlabs_auto_mode()
}

impl Default for ElevenLabsSettings {
    fn default() -> Self {
        Self {
            elevenlabs_api_key: String::new(),
            elevenlabs_voice_id: String::new(),
            elevenlabs_voices: Vec::new(),
            elevenlabs_models: Vec::new(),
            elevenlabs_tts_model: default_elevenlabs_tts_model(),
            elevenlabs_stability: default_elevenlabs_stability(),
            elevenlabs_similarity_boost: default_elevenlabs_similarity_boost(),
            elevenlabs_speed: default_elevenlabs_speed(),
            elevenlabs_use_speaker_boost: default_elevenlabs_use_speaker_boost(),
            elevenlabs_chunk_schedule_preset: default_elevenlabs_chunk_schedule_preset(),
            elevenlabs_tts_language_auto: default_elevenlabs_tts_language_auto(),
            elevenlabs_tts_language_code: String::new(),
            elevenlabs_tts_synthesis_mode: default_elevenlabs_tts_synthesis_mode(),
            elevenlabs_auto_mode: default_elevenlabs_auto_mode(),
            unified_outbound_topology: default_unified_outbound_topology(),
            elevenlabs_inbound_voice_id: String::new(),
            elevenlabs_inbound_tts_model: default_elevenlabs_tts_model(),
            elevenlabs_inbound_stability: default_elevenlabs_stability(),
            elevenlabs_inbound_similarity_boost: default_elevenlabs_similarity_boost(),
            elevenlabs_inbound_tts_synthesis_mode: default_elevenlabs_tts_synthesis_mode(),
        }
    }
}
