use serde_json::Value;

pub use crate::providers::shared::live::{decode_ws_message, LiveSetupOptions, TranscriptEvent};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::gemini::config::UPLOAD_SAMPLE_RATE;
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn decodes_binary_setup_complete_payload() {
        let payload = br#"{"setupComplete": {}}"#;
        let text = decode_ws_message(Message::Binary(payload.to_vec())).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert!(value.get("setupComplete").is_some());
    }

    #[test]
    fn build_audio_message_uses_upload_sample_rate() {
        let pcm = vec![0i16, 1000, -1000, 2000];
        let msg = build_audio_message(&pcm, UPLOAD_SAMPLE_RATE);
        assert!(msg.contains("audio/pcm;rate=16000"));
    }

    #[test]
    fn build_audio_message_encodes_pcm_as_base64() {
        let pcm = vec![1000i16, -1000i16];
        let msg = build_audio_message(&pcm, UPLOAD_SAMPLE_RATE);
        let value: Value = serde_json::from_str(&msg).unwrap();
        let data = value
            .pointer("/realtimeInput/audio/data")
            .and_then(|v| v.as_str())
            .expect("audio data field");
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data).unwrap();
        let expected: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn parse_api_error_reads_error_message() {
        let value: Value =
            serde_json::from_str(r#"{"error":{"message":"quota exceeded"}}"#).unwrap();
        assert_eq!(parse_api_error(&value).as_deref(), Some("quota exceeded"));
    }

    #[test]
    fn parse_api_error_reads_status_pointer() {
        let value: Value =
            serde_json::from_str(r#"{"status":{"message":"stream closed"}}"#).unwrap();
        assert_eq!(parse_api_error(&value).as_deref(), Some("stream closed"));
    }

    #[test]
    fn parse_server_message_setup_complete() {
        let (setup, audio, transcripts, turn_complete) =
            parse_server_message(r#"{"setupComplete": {}}"#, "outbound");
        assert!(setup);
        assert!(audio.is_empty());
        assert!(transcripts.is_empty());
        assert!(!turn_complete);
    }

    #[test]
    fn build_setup_message_uses_supported_gemini_vad_enums() {
        use crate::config::{AppConfig, VadSensitivity};
        let config = AppConfig {
            vad_start_sensitivity: VadSensitivity::Medium,
            vad_end_sensitivity: VadSensitivity::Medium,
            ..AppConfig::default()
        };
        let options = config.setup_options();
        let msg = build_setup_message("en", &options);
        assert!(!msg.contains("SENSITIVITY_MEDIUM"));
        assert!(msg.contains("START_SENSITIVITY_HIGH"));
        assert!(msg.contains("END_SENSITIVITY_LOW"));
    }

    #[test]
    fn parse_server_message_invalid_json_returns_empty() {
        let (setup, audio, transcripts, turn_complete) =
            parse_server_message("not-json", "inbound");
        assert!(!setup);
        assert!(audio.is_empty());
        assert!(transcripts.is_empty());
        assert!(!turn_complete);
    }

    #[test]
    fn parse_server_message_extracts_audio_and_interim_transcript() {
        let samples = vec![1000i16, -500i16];
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
        let payload = format!(
            r#"{{
                "serverContent": {{
                    "modelTurn": {{
                        "parts": [{{"inlineData": {{"data": "{encoded}"}}}}]
                    }},
                    "inputTranscription": {{"text": "hello"}},
                    "outputTranscription": {{"text": "xin chào"}}
                }}
            }}"#
        );

        let (setup, audio, transcripts, turn_complete) = parse_server_message(&payload, "outbound");

        assert!(!setup);
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].pcm_24k, samples);
        assert_eq!(transcripts.len(), 1);
        assert_eq!(transcripts[0].direction, "outbound");
        assert_eq!(transcripts[0].source_text.as_deref(), Some("hello"));
        assert_eq!(transcripts[0].translated_text.as_deref(), Some("xin chào"));
        assert!(transcripts[0].interim);
        assert!(!turn_complete);
    }

    #[test]
    fn parse_server_message_turn_complete_without_text() {
        let payload = r#"{"serverContent":{"turnComplete":true}}"#;
        let (_, _, transcripts, turn_complete) = parse_server_message(payload, "inbound");
        assert!(turn_complete);
        assert_eq!(transcripts.len(), 1);
        assert!(transcripts[0].turn_complete);
        assert!(!transcripts[0].interim);
        assert!(transcripts[0].source_text.is_none());
    }
}

pub fn ws_url() -> String {
    super::config::live_ws_url()
}

pub fn build_setup_message(target_language: &str, options: &LiveSetupOptions) -> String {
    let setup = serde_json::json!({
        "setup": {
            "model": format!("models/{}", options.model),
            "inputAudioTranscription": {},
            "outputAudioTranscription": {},
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "translationConfig": {
                    "targetLanguageCode": target_language,
                    "echoTargetLanguage": options.echo_target_language
                }
            },
            "realtimeInputConfig": {
                "automaticActivityDetection": {
                    "disabled": false,
                    "startOfSpeechSensitivity": options.vad_start_sensitivity,
                    "endOfSpeechSensitivity": options.vad_end_sensitivity,
                    "prefixPaddingMs": 20,
                    "silenceDurationMs": options.vad_silence_duration_ms
                }
            }
        }
    });
    setup.to_string()
}

pub fn build_audio_message(pcm: &[i16], sample_rate: u32) -> String {
    let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    serde_json::json!({
        "realtimeInput": {
            "audio": {
                "mimeType": format!("audio/pcm;rate={sample_rate}"),
                "data": data
            }
        }
    })
    .to_string()
}

#[derive(Debug, Clone)]
pub struct GeminiAudioChunk {
    pub pcm_24k: Vec<i16>,
}

pub fn parse_api_error(value: &Value) -> Option<String> {
    value
        .get("error")
        .map(|e| {
            e.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| e.to_string())
        })
        .or_else(|| {
            value
                .pointer("/status/message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
}

pub fn parse_server_message(
    text: &str,
    direction: &str,
) -> (bool, Vec<GeminiAudioChunk>, Vec<TranscriptEvent>, bool) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return (false, Vec::new(), Vec::new(), false);
    };
    parse_server_message_from_value(&value, direction)
}

pub fn parse_server_message_from_value(
    value: &Value,
    direction: &str,
) -> (bool, Vec<GeminiAudioChunk>, Vec<TranscriptEvent>, bool) {
    let mut setup_complete = false;
    let mut audio_chunks = Vec::new();
    let mut transcripts = Vec::new();

    if value.get("setupComplete").is_some() {
        setup_complete = true;
        return (setup_complete, audio_chunks, transcripts, false);
    }

    let Some(server_content) = value.get("serverContent") else {
        return (false, audio_chunks, transcripts, false);
    };

    if let Some(parts) = server_content
        .get("modelTurn")
        .and_then(|m| m.get("parts"))
        .and_then(|p| p.as_array())
    {
        for part in parts {
            if let Some(data) = part
                .get("inlineData")
                .and_then(|i| i.get("data"))
                .and_then(|d| d.as_str())
            {
                if let Ok(bytes) =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                {
                    let samples: Vec<i16> = bytes
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| i16::from_le_bytes(*c))
                        .collect();
                    if !samples.is_empty() {
                        audio_chunks.push(GeminiAudioChunk { pcm_24k: samples });
                    }
                }
            }
        }
    }

    let turn_complete = server_content
        .get("turnComplete")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let input_finished = server_content
        .get("inputTranscription")
        .and_then(|t| t.get("finished"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let output_finished = server_content
        .get("outputTranscription")
        .and_then(|t| t.get("finished"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Segment `finished` flags are informational only — committing a turn must wait
    // for `turnComplete` so mid-sentence fragments are not split into separate UI blocks.
    let interim = !turn_complete;

    let mut source_text = None;
    let mut translated_text = None;

    if let Some(text) = server_content
        .get("inputTranscription")
        .and_then(|t| t.get("text"))
        .and_then(|t| t.as_str())
    {
        source_text = Some(text.to_string());
    }

    if let Some(text) = server_content
        .get("outputTranscription")
        .and_then(|t| t.get("text"))
        .and_then(|t| t.as_str())
    {
        translated_text = Some(text.to_string());
    }

    if source_text.is_some() || translated_text.is_some() {
        transcripts.push(TranscriptEvent {
            direction: direction.to_string(),
            source_text,
            translated_text,
            interim,
            turn_complete,
            input_segment_finished: input_finished,
            output_segment_finished: output_finished,
            connection_gap: false,
            replace_live: false,
            live_source: None,
            live_translated: None,
        });
    } else if turn_complete {
        transcripts.push(TranscriptEvent {
            direction: direction.to_string(),
            source_text: None,
            translated_text: None,
            interim: false,
            turn_complete: true,
            input_segment_finished: false,
            output_segment_finished: false,
            connection_gap: false,
            replace_live: false,
            live_source: None,
            live_translated: None,
        });
    }

    (setup_complete, audio_chunks, transcripts, turn_complete)
}
