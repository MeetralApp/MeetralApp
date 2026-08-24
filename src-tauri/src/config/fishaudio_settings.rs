use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FishAudioLatency {
    Low,
    #[default]
    Balanced,
    Normal,
}

impl FishAudioLatency {
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Balanced => "balanced",
            Self::Normal => "normal",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FishAudioSettings {
    #[serde(default)]
    pub fishaudio_api_key: String,
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

pub(crate) fn default_fishaudio_tts_model() -> String {
    crate::providers::fishaudio::config::DEFAULT_TTS_MODEL.to_string()
}

pub(crate) fn default_fishaudio_temperature() -> f32 {
    crate::providers::fishaudio::config::DEFAULT_TEMPERATURE
}

pub(crate) fn default_fishaudio_speed() -> f32 {
    crate::providers::fishaudio::config::DEFAULT_SPEED
}

pub(crate) fn default_fishaudio_top_p() -> f32 {
    crate::providers::fishaudio::config::DEFAULT_TOP_P
}

impl Default for FishAudioSettings {
    fn default() -> Self {
        Self {
            fishaudio_api_key: String::new(),
            fishaudio_voice_id: String::new(),
            fishaudio_inbound_voice_id: String::new(),
            fishaudio_voices: Vec::new(),
            fishaudio_models: Vec::new(),
            fishaudio_tts_model: default_fishaudio_tts_model(),
            fishaudio_inbound_tts_model: default_fishaudio_tts_model(),
            fishaudio_latency: FishAudioLatency::Balanced,
            fishaudio_inbound_latency: FishAudioLatency::Balanced,
            fishaudio_temperature: default_fishaudio_temperature(),
            fishaudio_inbound_temperature: default_fishaudio_temperature(),
            fishaudio_speed: default_fishaudio_speed(),
            fishaudio_top_p: default_fishaudio_top_p(),
        }
    }
}
