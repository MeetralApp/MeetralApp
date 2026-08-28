use serde::{Deserialize, Serialize};

use crate::providers::shared::live::{SonioxContextPayload, SonioxContextProfile};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxSettings {
    #[serde(default)]
    pub soniox_always_on: SonioxContextPayload,
    #[serde(default)]
    pub soniox_context_profiles: Vec<SonioxContextProfile>,
    #[serde(default)]
    pub soniox_active_context_profile_id: Option<String>,
    #[serde(default = "default_soniox_tts_voice_field")]
    pub soniox_tts_voice: String,
    /// You → Meeting Soniox TTS voice when Engine voice is selected.
    /// Empty on load → normalized from [`Self::soniox_tts_voice`].
    #[serde(default)]
    pub soniox_tts_outbound_voice: String,
    #[serde(default = "default_soniox_tts_model_field")]
    pub soniox_tts_outbound_model: String,
    /// Meeting → You Soniox TTS model when Engine voice is selected.
    /// Empty on load → normalized from [`Self::soniox_tts_outbound_model`].
    #[serde(default)]
    pub soniox_tts_inbound_model: String,
    #[serde(default)]
    pub soniox_tts_voices: Vec<crate::voice::SonioxVoiceOption>,
    #[serde(default)]
    pub soniox_tts_models: Vec<crate::voice::SonioxTtsModelOption>,
    #[serde(default = "default_soniox_tts_speed_field")]
    pub soniox_tts_inbound_speed: f32,
    #[serde(default = "default_soniox_tts_speed_field")]
    pub soniox_tts_outbound_speed: f32,
    #[serde(default = "default_soniox_endpoint_latency_level")]
    pub soniox_endpoint_latency_adjustment_level: u8,
    #[serde(default = "default_soniox_endpoint_sensitivity")]
    pub soniox_endpoint_sensitivity: f64,
    #[serde(default = "default_soniox_max_endpoint_delay_ms")]
    pub soniox_max_endpoint_delay_ms: u32,
}

pub(crate) fn default_soniox_tts_voice_field() -> String {
    crate::providers::soniox::tts::config::default_soniox_tts_voice()
}

pub(crate) fn default_soniox_tts_model_field() -> String {
    crate::providers::soniox::tts::config::default_soniox_tts_model()
}

pub(crate) fn default_soniox_tts_speed_field() -> f32 {
    crate::providers::soniox::tts::config::default_soniox_tts_speed()
}

pub(crate) fn default_soniox_endpoint_latency_level() -> u8 {
    crate::providers::soniox::config::ENDPOINT_LATENCY_ADJUSTMENT_LEVEL
}

pub(crate) fn default_soniox_endpoint_sensitivity() -> f64 {
    crate::providers::soniox::config::ENDPOINT_SENSITIVITY
}

pub(crate) fn default_soniox_max_endpoint_delay_ms() -> u32 {
    crate::providers::soniox::config::MAX_ENDPOINT_DELAY_MS
}

impl Default for SonioxSettings {
    fn default() -> Self {
        Self {
            soniox_always_on: SonioxContextPayload::default(),
            soniox_context_profiles: Vec::new(),
            soniox_active_context_profile_id: None,
            soniox_tts_voice: default_soniox_tts_voice_field(),
            soniox_tts_outbound_voice: String::new(),
            soniox_tts_outbound_model: default_soniox_tts_model_field(),
            soniox_tts_inbound_model: String::new(),
            soniox_tts_voices: Vec::new(),
            soniox_tts_models: Vec::new(),
            soniox_tts_inbound_speed: default_soniox_tts_speed_field(),
            soniox_tts_outbound_speed: default_soniox_tts_speed_field(),
            soniox_endpoint_latency_adjustment_level: default_soniox_endpoint_latency_level(),
            soniox_endpoint_sensitivity: default_soniox_endpoint_sensitivity(),
            soniox_max_endpoint_delay_ms: default_soniox_max_endpoint_delay_ms(),
        }
    }
}
