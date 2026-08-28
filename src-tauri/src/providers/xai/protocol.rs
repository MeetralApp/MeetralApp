use base64::Engine as _;
use serde::{Deserialize, Serialize};

use super::config::{clamp_optimize_streaming_latency, clamp_speed};

#[derive(Debug, Clone)]
pub struct XaiInitSettings {
    pub voice_id: String,
    pub language: String,
    pub speed: f32,
    pub latency: crate::config::XaiLatency,
}

impl XaiInitSettings {
    pub fn optimize_streaming_latency(&self) -> u8 {
        clamp_optimize_streaming_latency(self.latency.as_optimize_level())
    }

    pub fn clamped_speed(&self) -> f32 {
        clamp_speed(self.speed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAudio {
    pub samples: Vec<i16>,
}

#[derive(Debug)]
pub enum ParsedServer {
    Audio(ParsedAudio),
    AudioDone,
    AudioClear,
    Error(String),
    Ignored,
}

#[derive(Serialize)]
struct TextDelta<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    delta: &'a str,
}

#[derive(Serialize)]
struct NamedEvent<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
}

#[derive(Deserialize)]
struct IncomingEvent {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

pub fn pack_text_delta(text: &str) -> anyhow::Result<String> {
    Ok(serde_json::to_string(&TextDelta {
        msg_type: "text.delta",
        delta: text,
    })?)
}

pub fn pack_text_done() -> anyhow::Result<String> {
    Ok(serde_json::to_string(&NamedEvent {
        msg_type: "text.done",
    })?)
}

pub fn pack_text_clear() -> anyhow::Result<String> {
    Ok(serde_json::to_string(&NamedEvent {
        msg_type: "text.clear",
    })?)
}

pub fn parse_server_message(text: &str) -> Option<ParsedServer> {
    let incoming: IncomingEvent = serde_json::from_str(text).ok()?;
    match incoming.msg_type.as_str() {
        "audio.delta" => {
            let Some(b64) = incoming.delta.filter(|s| !s.is_empty()) else {
                return Some(ParsedServer::Audio(ParsedAudio {
                    samples: Vec::new(),
                }));
            };
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64.as_bytes())
                .ok()?;
            if bytes.is_empty() {
                return Some(ParsedServer::Audio(ParsedAudio {
                    samples: Vec::new(),
                }));
            }
            if !bytes.len().is_multiple_of(2) {
                return None;
            }
            let samples: Vec<i16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| i16::from_le_bytes(*chunk))
                .collect();
            Some(ParsedServer::Audio(ParsedAudio { samples }))
        }
        "audio.done" => Some(ParsedServer::AudioDone),
        "audio.clear" => Some(ParsedServer::AudioClear),
        "error" => Some(ParsedServer::Error(
            incoming
                .message
                .unwrap_or_else(|| "xAI TTS server error".into()),
        )),
        _ => Some(ParsedServer::Ignored),
    }
}

pub fn user_message_for_xai_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("unauthorized")
        || (lower.contains("invalid") && lower.contains("key"))
    {
        return "xAI API key is invalid — check Settings → Voice.".into();
    }
    if lower.contains("404") || lower.contains("voice not") || lower.contains("unknown voice") {
        return "Voice not found — pick a voice in Settings → Voice.".into();
    }
    if lower.contains("429") || lower.contains("rate limit") {
        return "xAI rate limit — wait and retry.".into();
    }
    if lower.contains("503") || lower.contains("unavailable") {
        return "xAI TTS is temporarily unavailable — retry.".into();
    }
    if lower.contains("timeout") {
        return "xAI connect timeout (15s).".into();
    }
    format!("xAI error: {raw}")
}

pub fn is_non_retryable_xai_error(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("401")
        || lower.contains("unauthorized")
        || (lower.contains("invalid") && lower.contains("key"))
        || lower.contains("404")
        || lower.contains("voice not")
        || lower.contains("unknown voice")
        || lower.contains("400") && lower.contains("language")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_client_events() {
        let delta = pack_text_delta("Hello").unwrap();
        assert!(delta.contains("\"type\":\"text.delta\""));
        assert!(delta.contains("\"delta\":\"Hello\""));
        assert!(pack_text_done().unwrap().contains("text.done"));
        assert!(pack_text_clear().unwrap().contains("text.clear"));
    }

    #[test]
    fn parse_audio_delta_base64_le_i16() {
        let pcm = [1i16, -2i16]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect::<Vec<_>>();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&pcm);
        let msg = format!(r#"{{"type":"audio.delta","delta":"{b64}"}}"#);
        match parse_server_message(&msg).unwrap() {
            ParsedServer::Audio(audio) => assert_eq!(audio.samples, vec![1, -2]),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_done_clear_error() {
        assert!(matches!(
            parse_server_message(r#"{"type":"audio.done"}"#),
            Some(ParsedServer::AudioDone)
        ));
        assert!(matches!(
            parse_server_message(r#"{"type":"audio.clear"}"#),
            Some(ParsedServer::AudioClear)
        ));
        match parse_server_message(r#"{"type":"error","message":"boom"}"#).unwrap() {
            ParsedServer::Error(msg) => assert_eq!(msg, "boom"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn user_message_401_404() {
        assert!(user_message_for_xai_error("401 unauthorized").contains("API key"));
        assert!(user_message_for_xai_error("404 unknown voice").contains("Voice not"));
        assert!(is_non_retryable_xai_error("401 unauthorized"));
        assert!(is_non_retryable_xai_error("404 voice not found"));
        assert!(!is_non_retryable_xai_error("network timeout"));
    }
}
