use base64::Engine;
use serde_json::{json, Value};

use super::config::{clamp_soniox_tts_speed, DEFAULT_SONIOX_TTS_MODEL, TTS_SAMPLE_RATE};

pub fn build_tts_config_message(
    api_key: &str,
    language: &str,
    voice: &str,
    stream_id: &str,
    model: &str,
    speed: f32,
) -> String {
    json!({
        "api_key": api_key,
        "model": if model.trim().is_empty() { DEFAULT_SONIOX_TTS_MODEL } else { model },
        "language": language,
        "voice": voice,
        "audio_format": "pcm_s16le",
        "sample_rate": TTS_SAMPLE_RATE,
        "speed": clamp_soniox_tts_speed(speed),
        "stream_id": stream_id,
    })
    .to_string()
}

pub fn build_text_message(stream_id: &str, text: &str, text_end: bool) -> String {
    json!({
        "text": text,
        "text_end": text_end,
        "stream_id": stream_id,
    })
    .to_string()
}

pub fn build_cancel_message(stream_id: &str) -> String {
    json!({
        "stream_id": stream_id,
        "cancel": true,
    })
    .to_string()
}

pub fn build_keepalive_message() -> String {
    // Soniox TTS docs: {"keep_alive": true} (underscore).
    json!({ "keep_alive": true }).to_string()
}

#[derive(Debug)]
pub struct ParsedTtsAudio {
    pub stream_id: Option<String>,
    pub samples: Vec<i16>,
    pub audio_end: bool,
    pub terminated: bool,
    pub error: Option<String>,
}

pub fn parse_tts_message(text: &str) -> Option<ParsedTtsAudio> {
    let value: Value = serde_json::from_str(text).ok()?;

    let stream_id = value
        .get("stream_id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let error = value
        .get("error_message")
        .and_then(|m| m.as_str())
        .map(|m| m.to_string())
        .or_else(|| {
            value
                .get("error_type")
                .and_then(|t| t.as_str())
                .map(|t| t.to_string())
        });

    let terminated = value
        .get("terminated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let audio_end = value
        .get("audio_end")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let samples = value
        .get("audio")
        .and_then(|a| a.as_str())
        .and_then(decode_pcm_s16le_b64)
        .unwrap_or_default();

    if samples.is_empty() && !audio_end && !terminated && error.is_none() {
        return None;
    }

    Some(ParsedTtsAudio {
        stream_id,
        samples,
        audio_end,
        terminated,
        error,
    })
}

fn decode_pcm_s16le_b64(b64: &str) -> Option<Vec<i16>> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    if bytes.len() < 2 {
        return Some(Vec::new());
    }
    let mut samples = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.as_chunks::<2>().0 {
        samples.push(i16::from_le_bytes(*chunk));
    }
    Some(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_includes_pcm_24k_and_speed() {
        let msg = build_tts_config_message("key", "en", "Adrian", "s1", "tts-rt-v1", 1.2);
        let v: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["audio_format"], "pcm_s16le");
        assert_eq!(v["sample_rate"], 24000);
        assert_eq!(v["voice"], "Adrian");
        assert!((v["speed"].as_f64().unwrap() - 1.2).abs() < 1e-6);
    }

    #[test]
    fn parses_audio_chunk() {
        let pcm = i16::to_le_bytes(1000);
        let b64 = base64::engine::general_purpose::STANDARD.encode(pcm);
        let msg = format!(r#"{{"audio":"{b64}","stream_id":"s1"}}"#);
        let parsed = parse_tts_message(&msg).unwrap();
        assert_eq!(parsed.samples, vec![1000]);
        assert!(!parsed.terminated);
    }

    #[test]
    fn parses_stream_id_on_audio_and_terminated() {
        let pcm = i16::to_le_bytes(1000);
        let b64 = base64::engine::general_purpose::STANDARD.encode(pcm);
        let msg = format!(r#"{{"audio":"{b64}","stream_id":"tts-abc"}}"#);
        let parsed = parse_tts_message(&msg).unwrap();
        assert_eq!(parsed.stream_id.as_deref(), Some("tts-abc"));

        let parsed = parse_tts_message(r#"{"terminated":true,"stream_id":"tts-xyz"}"#).unwrap();
        assert!(parsed.terminated);
        assert_eq!(parsed.stream_id.as_deref(), Some("tts-xyz"));
    }

    #[test]
    fn parses_error_stream_id() {
        let parsed = parse_tts_message(
            r#"{"stream_id":"tts-old","error_code":400,"error_type":"invalid_request","error_message":"Stream tts-old has already been cancelled. Start a new stream to send more text."}"#,
        )
        .unwrap();
        assert_eq!(parsed.stream_id.as_deref(), Some("tts-old"));
        assert!(parsed.error.is_some());
    }

    #[test]
    fn keepalive_uses_keep_alive_field() {
        let msg = build_keepalive_message();
        let v: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["keep_alive"], true);
        assert!(v.get("keepalive").is_none());
    }
}
