use base64::Engine;
use serde_json::{json, Value};

use super::config::elevenlabs_speed_for_api;

#[derive(Debug, Clone)]
pub struct ElevenLabsInitSettings {
    pub stability: f32,
    pub similarity_boost: f32,
    pub speed: f32,
    pub use_speaker_boost: bool,
    /// `None` omits `generation_config` from the WS init message (Natural mode).
    pub chunk_schedule: Option<[u32; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAudio {
    pub samples: Vec<i16>,
    pub is_final: bool,
}

pub fn build_init_message(settings: &ElevenLabsInitSettings) -> String {
    let mut payload = json!({
        "text": " ",
        "voice_settings": {
            "stability": settings.stability,
            "similarity_boost": settings.similarity_boost,
            "speed": elevenlabs_speed_for_api(settings.speed),
            "use_speaker_boost": settings.use_speaker_boost,
        },
    });
    if let Some(schedule) = settings.chunk_schedule {
        payload["generation_config"] = json!({
            "chunk_length_schedule": schedule,
        });
    }
    payload.to_string()
}

pub fn build_text_chunk(text: &str, trigger_generation: bool) -> String {
    let payload = if text.ends_with(' ') {
        text.to_string()
    } else {
        format!("{text} ")
    };
    json!({
        "text": payload,
        "try_trigger_generation": trigger_generation,
    })
    .to_string()
}

pub fn build_flush_message() -> String {
    json!({
        "text": " ",
        "flush": true,
    })
    .to_string()
}

pub fn build_close_message() -> String {
    json!({ "text": "" }).to_string()
}

pub fn parse_audio_message(text: &str) -> Option<ParsedAudio> {
    let value: Value = serde_json::from_str(text).ok()?;
    let is_final = value
        .get("isFinal")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let Some(data) = value.get("audio").and_then(|v| v.as_str()) else {
        return is_final.then(|| ParsedAudio {
            samples: Vec::new(),
            is_final: true,
        });
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    if bytes.len() < 2 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let samples: Vec<i16> = bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    Some(ParsedAudio { samples, is_final })
}

pub fn parse_error_message(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
        return Some(err.to_string());
    }
    if let Some(detail) = value.get("detail") {
        if let Some(status) = detail.get("status").and_then(|v| v.as_str()) {
            return Some(status.to_string());
        }
        if let Some(msg) = detail.get("message").and_then(|v| v.as_str()) {
            return Some(msg.to_string());
        }
    }
    if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        if value.get("audio").is_none() {
            return Some(msg.to_string());
        }
    }
    None
}

/// Map ElevenLabs API / WS error tokens to user-facing copy.
pub fn user_message_for_elevenlabs_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("voice_id_does_not_exist") || lower.contains("voice not found") {
        return "Voice not in your ElevenLabs account — pick from My Voices in Settings.".into();
    }
    if lower.contains("invalid_api_key") {
        return "ElevenLabs API key is invalid — check Settings → Voice.".into();
    }
    if lower.contains("invalid_voice_settings") {
        return "Voice settings rejected by ElevenLabs — try Speed between 0.7 and 1.19, or reset voice sliders to defaults.".into();
    }
    if lower.contains("1006") || lower.contains("abnormal") {
        return "Custom voice connection lost — Stop and Start outbound to retry.".into();
    }
    format!("Custom voice error: {raw}")
}

pub fn is_non_retryable_elevenlabs_error(raw: &str) -> bool {
    let lower = raw.to_lowercase();
    lower.contains("voice_id_does_not_exist")
        || lower.contains("invalid_api_key")
        || lower.contains("invalid_voice_settings")
        || lower.contains("voice not found")
        || lower.contains("quota_exceeded")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> ElevenLabsInitSettings {
        ElevenLabsInitSettings {
            stability: 0.5,
            similarity_boost: 0.75,
            speed: 1.0,
            use_speaker_boost: true,
            chunk_schedule: Some([50, 55, 60, 65]),
        }
    }

    #[test]
    fn init_message_omits_generation_config_when_schedule_unset() {
        let msg = build_init_message(&ElevenLabsInitSettings {
            chunk_schedule: None,
            ..sample_settings()
        });
        let value: Value = serde_json::from_str(&msg).unwrap();
        assert!(value.get("generation_config").is_none());
        assert!(value.pointer("/voice_settings/stability").is_some());
    }

    #[test]
    fn init_message_includes_voice_and_chunk_settings() {
        let msg = build_init_message(&sample_settings());
        let value: Value = serde_json::from_str(&msg).unwrap();
        let schedule = value
            .pointer("/generation_config/chunk_length_schedule")
            .and_then(|v| v.as_array())
            .expect("schedule");
        assert_eq!(schedule.len(), 4);
        assert_eq!(schedule[0].as_u64(), Some(50));
        assert_eq!(
            value
                .pointer("/voice_settings/speed")
                .and_then(|v| v.as_f64()),
            Some(1.0)
        );
        assert_eq!(
            value
                .pointer("/voice_settings/use_speaker_boost")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn init_message_speed_at_slider_max_stays_within_api() {
        let msg = build_init_message(&ElevenLabsInitSettings {
            speed: 1.2,
            chunk_schedule: None,
            ..sample_settings()
        });
        let value: Value = serde_json::from_str(&msg).unwrap();
        let speed = value
            .pointer("/voice_settings/speed")
            .and_then(|v| v.as_f64())
            .expect("speed");
        assert!(speed <= 1.2);
        assert!((speed - 1.2).abs() < f64::EPSILON);
        assert!(!msg.contains("1.200000"));
    }

    #[test]
    fn parses_pcm_audio_chunk() {
        let samples = vec![1000i16, -500i16];
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let payload = format!(r#"{{"audio":"{encoded}"}}"#);
        let parsed = parse_audio_message(&payload).expect("audio");
        assert_eq!(parsed.samples, samples);
        assert!(!parsed.is_final);
    }

    #[test]
    fn parses_is_final_without_audio() {
        let parsed = parse_audio_message(r#"{"isFinal":true}"#).expect("final");
        assert!(parsed.is_final);
        assert!(parsed.samples.is_empty());
    }

    #[test]
    fn flush_message_sets_flag() {
        let msg = build_flush_message();
        let value: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(value.get("flush").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn text_chunk_appends_trailing_space() {
        let msg = build_text_chunk("Hello", false);
        let value: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(value.get("text").and_then(|v| v.as_str()), Some("Hello "));
    }

    #[test]
    fn text_chunk_trigger_generation_flag() {
        let interim = build_text_chunk("Hello", false);
        let interim_val: Value = serde_json::from_str(&interim).unwrap();
        assert_eq!(
            interim_val
                .get("try_trigger_generation")
                .and_then(|v| v.as_bool()),
            Some(false)
        );

        let triggered = build_text_chunk("Hello", true);
        let triggered_val: Value = serde_json::from_str(&triggered).unwrap();
        assert_eq!(
            triggered_val
                .get("try_trigger_generation")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn user_message_maps_voice_not_found() {
        let msg = user_message_for_elevenlabs_error("voice_id_does_not_exist");
        assert!(msg.contains("My Voices"));
    }

    #[test]
    fn non_retryable_detects_invalid_key() {
        assert!(is_non_retryable_elevenlabs_error("invalid_api_key"));
        assert!(is_non_retryable_elevenlabs_error("invalid_voice_settings"));
        assert!(!is_non_retryable_elevenlabs_error("network timeout"));
    }
}
