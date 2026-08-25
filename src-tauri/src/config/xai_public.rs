//! Public xAI fields exposed on [`ConfigView`] (no API key).

use serde::{Deserialize, Serialize};

use super::xai_settings::{default_xai_speed, default_xai_voice_id, XaiLatency, XaiSettings};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct XaiPublicSettings {
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

impl From<&XaiSettings> for XaiPublicSettings {
    fn from(s: &XaiSettings) -> Self {
        Self {
            xai_voice_id: s.xai_voice_id.clone(),
            xai_inbound_voice_id: s.xai_inbound_voice_id.clone(),
            xai_voices: s.xai_voices.clone(),
            xai_latency: s.xai_latency,
            xai_inbound_latency: s.xai_inbound_latency,
            xai_outbound_speed: s.xai_outbound_speed,
            xai_inbound_speed: s.xai_inbound_speed,
        }
    }
}

impl XaiPublicSettings {
    pub fn merge_into(&self, existing: &XaiSettings) -> XaiSettings {
        XaiSettings {
            xai_api_key: existing.xai_api_key.clone(),
            xai_voice_id: self.xai_voice_id.clone(),
            xai_inbound_voice_id: self.xai_inbound_voice_id.clone(),
            xai_voices: self.xai_voices.clone(),
            xai_latency: self.xai_latency,
            xai_inbound_latency: self.xai_inbound_latency,
            xai_outbound_speed: self.xai_outbound_speed,
            xai_inbound_speed: self.xai_inbound_speed,
        }
    }
}
