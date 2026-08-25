use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum XaiLatency {
    Low,
    #[default]
    Balanced,
    Normal,
}

impl XaiLatency {
    /// Maps to xAI `optimize_streaming_latency` (0=best quality, 2=lowest TTFA).
    pub fn as_optimize_level(self) -> u8 {
        match self {
            Self::Low => 2,
            Self::Balanced => 1,
            Self::Normal => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XaiSettings {
    #[serde(default)]
    pub xai_api_key: String,
    #[serde(default = "default_xai_voice_id")]
    pub xai_voice_id: String,
    #[serde(default = "default_xai_voice_id")]
    pub xai_inbound_voice_id: String,
    #[serde(default)]
    pub xai_voices: Vec<crate::providers::xai::XaiVoiceOption>,
    #[serde(default)]
    pub xai_latency: XaiLatency,
    #[serde(default)]
    pub xai_inbound_latency: XaiLatency,
    #[serde(default = "default_xai_speed")]
    pub xai_outbound_speed: f32,
    #[serde(default = "default_xai_speed")]
    pub xai_inbound_speed: f32,
}

pub(crate) fn default_xai_voice_id() -> String {
    crate::providers::xai::config::default_voice_id()
}

pub(crate) fn default_xai_speed() -> f32 {
    crate::providers::xai::config::DEFAULT_SPEED
}

impl Default for XaiSettings {
    fn default() -> Self {
        Self {
            xai_api_key: String::new(),
            xai_voice_id: default_xai_voice_id(),
            xai_inbound_voice_id: default_xai_voice_id(),
            xai_voices: Vec::new(),
            xai_latency: XaiLatency::Balanced,
            xai_inbound_latency: XaiLatency::Balanced,
            xai_outbound_speed: default_xai_speed(),
            xai_inbound_speed: default_xai_speed(),
        }
    }
}
