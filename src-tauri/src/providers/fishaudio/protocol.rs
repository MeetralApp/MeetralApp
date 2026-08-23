use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::config::{
    clamp_speed, clamp_temperature, clamp_top_p, normalize_tts_model, FISH_PCM_SAMPLE_RATE,
};

#[derive(Debug, Clone)]
pub struct FishAudioInitSettings {
    pub reference_id: String,
    pub model_id: String,
    pub latency: crate::config::FishAudioLatency,
    pub temperature: f32,
    pub top_p: f32,
    pub speed: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAudio {
    pub samples: Vec<i16>,
    pub is_final: bool,
}

#[derive(Serialize)]
struct StartEvent<'a> {
    event: &'a str,
    request: StartRequest<'a>,
}

#[derive(Serialize)]
struct StartRequest<'a> {
    text: &'a str,
    reference_id: &'a str,
    format: &'a str,
    sample_rate: u32,
    latency: &'a str,
    temperature: f32,
    top_p: f32,
    normalize: bool,
    condition_on_previous_chunks: bool,
    prosody: Prosody,
}

#[derive(Serialize)]
struct Prosody {
    speed: f32,
    volume: i32,
    normalize_loudness: bool,
}

#[derive(Serialize)]
struct TextEvent<'a> {
    event: &'a str,
    text: &'a str,
}

#[derive(Serialize)]
struct NamedEvent<'a> {
    event: &'a str,
}

#[derive(Deserialize)]
struct IncomingEvent {
    event: String,
    #[serde(default)]
    audio: Option<serde_bytes::ByteBuf>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

pub fn pack_start_message(settings: &FishAudioInitSettings) -> anyhow::Result<Vec<u8>> {
    let payload = StartEvent {
        event: "start",
        request: StartRequest {
            text: "",
            reference_id: settings.reference_id.trim(),
            format: "pcm",
            sample_rate: FISH_PCM_SAMPLE_RATE,
            latency: settings.latency.as_api_str(),
            temperature: clamp_temperature(settings.temperature),
            top_p: clamp_top_p(settings.top_p),
            normalize: true,
            condition_on_previous_chunks: true,
            prosody: Prosody {
                speed: clamp_speed(settings.speed),
                volume: 0,
                normalize_loudness: true,
            },
        },
    };
    rmp_serde::to_vec_named(&payload).context("fishaudio pack start")
}

pub fn pack_text_message(text: &str) -> anyhow::Result<Vec<u8>> {
    let payload = if text.ends_with(' ') {
        text.to_string()
    } else {
        format!("{text} ")
    };
    rmp_serde::to_vec_named(&TextEvent {
        event: "text",
        text: &payload,
    })
    .context("fishaudio pack text")
}

pub fn pack_flush_message() -> anyhow::Result<Vec<u8>> {
    rmp_serde::to_vec_named(&NamedEvent { event: "flush" }).context("fishaudio pack flush")
}

pub fn pack_stop_message() -> anyhow::Result<Vec<u8>> {
    rmp_serde::to_vec_named(&NamedEvent { event: "stop" }).context("fishaudio pack stop")
}

pub fn parse_server_message(bytes: &[u8]) -> Option<ParsedServer> {
    let incoming: IncomingEvent = rmp_serde::from_slice(bytes).ok()?;
    match incoming.event.as_str() {
        "audio" => {
            let Some(buf) = incoming.audio else {
                return Some(ParsedServer::Audio(ParsedAudio {
                    samples: Vec::new(),
                    is_final: false,
                }));
            };
            let bytes = buf.as_slice();
            if bytes.len() < 2 || !bytes.len().is_multiple_of(2) {
                return None;
            }
            let samples: Vec<i16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| i16::from_le_bytes(*chunk))
                .collect();
            Some(ParsedServer::Audio(ParsedAudio {
                samples,
                is_final: false,
            }))
        }
        "finish" => Some(ParsedServer::Finish {
            reason: incoming.reason.unwrap_or_default(),
        }),
        other if other.eq_ignore_ascii_case("error") => Some(ParsedServer::Error(
            incoming
                .message
                .unwrap_or_else(|| "Fish Audio server error".into()),
        )),
        _ => Some(ParsedServer::Ignored),
    }
}

#[derive(Debug)]
pub enum ParsedServer {
    Audio(ParsedAudio),
    Finish { reason: String },
    Error(String),
    Ignored,
}

pub fn user_message_for_fishaudio_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("no permission")
        || lower.contains("unauthorized")
        || lower.contains("invalid") && lower.contains("key")
    {
        return "Fish Audio API key is invalid — check Settings → Voice.".into();
    }
    if lower.contains("402") || lower.contains("credit") || lower.contains("payment") {
        return "Fish Audio is out of credits.".into();
    }
    if lower.contains("429") || lower.contains("rate limit") {
        return "Fish Audio rate limit — wait and retry.".into();
    }
    if lower.contains("503") || lower.contains("high load") || lower.contains("busy") {
        return "Fish Audio is busy — retry.".into();
    }
    if lower.contains("reference not found")
        || lower.contains("reference_id")
        || lower.contains("voice not")
        || lower.contains("model not found")
    {
        return "Voice not in this Fish workspace — pick from My voices in Settings.".into();
    }
    if lower.contains("timeout") {
        return "Fish Audio connect timeout (15s).".into();
    }
    format!("Fish Audio error: {raw}")
}

pub fn is_non_retryable_fishaudio_error(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("401")
        || lower.contains("no permission")
        || lower.contains("unauthorized")
        || lower.contains("invalid") && lower.contains("key")
        || lower.contains("reference not found")
        || lower.contains("voice not")
}

pub fn model_header_value(model_id: &str) -> &'static str {
    normalize_tts_model(model_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FishAudioLatency;

    fn sample_settings() -> FishAudioInitSettings {
        FishAudioInitSettings {
            reference_id: "voice-1".into(),
            model_id: "s2.1-pro".into(),
            latency: FishAudioLatency::Balanced,
            temperature: 0.7,
            top_p: 0.7,
            speed: 1.0,
        }
    }

    #[test]
    fn start_event_is_pcm_24k() {
        let packed = pack_start_message(&sample_settings()).unwrap();
        #[derive(Deserialize)]
        struct StartWire {
            event: String,
            request: StartReq,
        }
        #[derive(Deserialize)]
        struct StartReq {
            format: String,
            sample_rate: u32,
        }
        let value: StartWire = rmp_serde::from_slice(&packed).unwrap();
        assert_eq!(value.event, "start");
        assert_eq!(value.request.format, "pcm");
        assert_eq!(value.request.sample_rate, 24000);
    }

    #[test]
    fn flush_and_stop_event_names() {
        #[derive(Deserialize)]
        struct Named {
            event: String,
        }
        let flush: Named = rmp_serde::from_slice(&pack_flush_message().unwrap()).unwrap();
        let stop: Named = rmp_serde::from_slice(&pack_stop_message().unwrap()).unwrap();
        assert_eq!(flush.event, "flush");
        assert_eq!(stop.event, "stop");
    }

    #[test]
    fn parse_audio_even_length_le_i16() {
        let samples: Vec<u8> = [1i16, -2i16].iter().flat_map(|s| s.to_le_bytes()).collect();
        #[derive(Serialize)]
        struct AudioOut<'a> {
            event: &'a str,
            #[serde(with = "serde_bytes")]
            audio: &'a [u8],
        }
        let packed = rmp_serde::to_vec_named(&AudioOut {
            event: "audio",
            audio: &samples,
        })
        .unwrap();
        let parsed = parse_server_message(&packed).unwrap();
        match parsed {
            ParsedServer::Audio(audio) => {
                assert_eq!(audio.samples, vec![1, -2]);
                assert!(!audio.is_final);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn user_message_401_402() {
        assert!(user_message_for_fishaudio_error("401 no permission").contains("API key"));
        assert!(user_message_for_fishaudio_error("402 payment").contains("credits"));
        assert!(is_non_retryable_fishaudio_error("401 unauthorized"));
        assert!(!is_non_retryable_fishaudio_error("network timeout"));
    }
}
