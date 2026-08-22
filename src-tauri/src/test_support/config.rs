use crate::config::{AppConfig, DeviceRef, PipelineOutputMode};

pub fn sample_config() -> AppConfig {
    AppConfig {
        gemini_api_key: "secret".into(),
        user_mic: DeviceRef {
            id: "mic".into(),
            name: "Mic".into(),
        },
        teams_mic_feed: DeviceRef {
            id: "teams".into(),
            name: "Teams".into(),
        },
        meeting_capture: DeviceRef {
            id: "cap".into(),
            name: "Capture".into(),
        },
        local_playback: DeviceRef {
            id: "hp".into(),
            name: "Headphones".into(),
        },
        outbound_mode: PipelineOutputMode::Translated,
        inbound_mode: PipelineOutputMode::Translated,
        ..AppConfig::default()
    }
}
