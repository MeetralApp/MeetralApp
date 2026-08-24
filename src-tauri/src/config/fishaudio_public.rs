//! Public Fish Audio fields exposed on [`ConfigView`] (no API key).

use serde::{Deserialize, Serialize};

use super::fishaudio_settings::{
    default_fishaudio_speed, default_fishaudio_temperature, default_fishaudio_top_p,
    default_fishaudio_tts_model, FishAudioLatency, FishAudioSettings,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FishAudioPublicSettings {
    #[serde(default)]
    pub fishaudio_voice_id: String,
    #[serde(default)]
    pub fishaudio_inbound_voice_id: String,
    #[serde(default)]
    pub fishaudio_voices: Vec<crate::providers::fishaudio::FishAudioVoiceOption>,
    #[serde(default)]
    pub fishaudio_models: Vec<crate::providers::fishaudio::FishAudioModelOption>,
    #[serde(default = "default_fishaudio_tts_model")]
    pub fishaudio_tts_model: String,
    #[serde(default = "default_fishaudio_tts_model")]
    pub fishaudio_inbound_tts_model: String,
    #[serde(default)]
    pub fishaudio_latency: FishAudioLatency,
    #[serde(default)]
    pub fishaudio_inbound_latency: FishAudioLatency,
    #[serde(default = "default_fishaudio_temperature")]
    pub fishaudio_temperature: f32,
    #[serde(default = "default_fishaudio_temperature")]
    pub fishaudio_inbound_temperature: f32,
    #[serde(default = "default_fishaudio_speed")]
    pub fishaudio_speed: f32,
    #[serde(default = "default_fishaudio_top_p")]
    pub fishaudio_top_p: f32,
}

impl From<&FishAudioSettings> for FishAudioPublicSettings {
    fn from(s: &FishAudioSettings) -> Self {
        Self {
            fishaudio_voice_id: s.fishaudio_voice_id.clone(),
            fishaudio_inbound_voice_id: s.fishaudio_inbound_voice_id.clone(),
            fishaudio_voices: s.fishaudio_voices.clone(),
            fishaudio_models: s.fishaudio_models.clone(),
            fishaudio_tts_model: s.fishaudio_tts_model.clone(),
            fishaudio_inbound_tts_model: s.fishaudio_inbound_tts_model.clone(),
            fishaudio_latency: s.fishaudio_latency,
            fishaudio_inbound_latency: s.fishaudio_inbound_latency,
            fishaudio_temperature: s.fishaudio_temperature,
            fishaudio_inbound_temperature: s.fishaudio_inbound_temperature,
            fishaudio_speed: s.fishaudio_speed,
            fishaudio_top_p: s.fishaudio_top_p,
        }
    }
}

impl FishAudioPublicSettings {
    pub fn merge_into(&self, existing: &FishAudioSettings) -> FishAudioSettings {
        FishAudioSettings {
            fishaudio_api_key: existing.fishaudio_api_key.clone(),
            fishaudio_voice_id: self.fishaudio_voice_id.clone(),
            fishaudio_inbound_voice_id: self.fishaudio_inbound_voice_id.clone(),
            fishaudio_voices: self.fishaudio_voices.clone(),
            fishaudio_models: self.fishaudio_models.clone(),
            fishaudio_tts_model: self.fishaudio_tts_model.clone(),
            fishaudio_inbound_tts_model: self.fishaudio_inbound_tts_model.clone(),
            fishaudio_latency: self.fishaudio_latency,
            fishaudio_inbound_latency: self.fishaudio_inbound_latency,
            fishaudio_temperature: self.fishaudio_temperature,
            fishaudio_inbound_temperature: self.fishaudio_inbound_temperature,
            fishaudio_speed: self.fishaudio_speed,
            fishaudio_top_p: self.fishaudio_top_p,
        }
    }
}
