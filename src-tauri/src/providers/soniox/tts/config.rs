use std::time::Duration;

pub const SONIOX_TTS_WS: &str = "wss://tts-rt.soniox.com/tts-websocket";
pub const TTS_SAMPLE_RATE: u32 = 24000;
pub const DEFAULT_SONIOX_TTS_MODEL: &str = "tts-rt-v1";
pub const DEFAULT_SONIOX_TTS_VOICE: &str = "Adrian";
/// Soniox TTS `speed` API range is 0.7–1.3; 1.0 is normal.
pub const DEFAULT_SONIOX_TTS_SPEED: f32 = 1.0;
pub const SONIOX_TTS_SPEED_MIN: f32 = 0.7;
pub const SONIOX_TTS_SPEED_MAX: f32 = 1.3;

/// End an open TTS stream if no text arrives for this long (keeps WebSocket).
/// Prevents Soniox `request_timeout` from idle mid-stream pauses.
pub const STREAM_IDLE_TEXT_END: Duration = Duration::from_secs(3);

/// Match ElevenLabs coalesce (~80 ms @ 24 kHz) so first PCM is not held longer.
pub const SONIOX_PCM_COALESCE_MIN_SAMPLES: usize = 1920;

pub fn default_soniox_tts_voice() -> String {
    DEFAULT_SONIOX_TTS_VOICE.to_string()
}

pub fn default_soniox_tts_model() -> String {
    DEFAULT_SONIOX_TTS_MODEL.to_string()
}

pub fn default_soniox_tts_speed() -> f32 {
    DEFAULT_SONIOX_TTS_SPEED
}

pub fn clamp_soniox_tts_speed(speed: f32) -> f32 {
    speed.clamp(SONIOX_TTS_SPEED_MIN, SONIOX_TTS_SPEED_MAX)
}

pub fn normalize_soniox_tts_voice(voice: &str) -> String {
    let trimmed = voice.trim();
    if trimmed.is_empty() {
        default_soniox_tts_voice()
    } else {
        trimmed.to_string()
    }
}

pub fn tts_ws_url() -> &'static str {
    SONIOX_TTS_WS
}

/// Built-in fallback when the TTS models API is unreachable.
pub const SONIOX_TTS_VOICES: &[(&str, &str)] = &[
    ("Adrian", "Male"),
    ("Maya", "Female"),
    ("Noah", "Male"),
    ("Emma", "Female"),
    ("Claire", "Female"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_voice_is_adrian() {
        assert_eq!(normalize_soniox_tts_voice(""), DEFAULT_SONIOX_TTS_VOICE);
        assert_eq!(normalize_soniox_tts_voice("Emily"), "Emily");
    }

    #[test]
    fn clamp_soniox_tts_speed_to_api_range() {
        assert!((clamp_soniox_tts_speed(0.5) - SONIOX_TTS_SPEED_MIN).abs() < f32::EPSILON);
        assert!((clamp_soniox_tts_speed(2.0) - SONIOX_TTS_SPEED_MAX).abs() < f32::EPSILON);
        assert!((clamp_soniox_tts_speed(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((clamp_soniox_tts_speed(1.2) - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn stream_idle_text_end_is_under_server_timeout() {
        // Must be shorter than Soniox request_timeout for idle mid-stream pauses.
        assert!(STREAM_IDLE_TEXT_END.as_secs() >= 2);
        assert!(STREAM_IDLE_TEXT_END.as_secs() <= 5);
    }
}
