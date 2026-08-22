use serde_json::{json, Value};

use super::config::UPLOAD_SAMPLE_RATE;
use super::context::{build_context_object, SonioxContextInput};
use crate::providers::shared::live::TranscriptEvent;

pub const END_TOKEN: &str = "<end>";

#[derive(Debug, Clone)]
pub struct SonioxSttSetup {
    pub api_key: String,
    pub model: String,
    pub target_language: String,
    pub language_hints: Vec<String>,
    pub context: SonioxContextInput,
    pub endpoint_latency_adjustment_level: u8,
    pub endpoint_sensitivity: f64,
    pub max_endpoint_delay_ms: u32,
    /// When false (Session Mode Notes), omit the `translation` one_way block.
    pub translation_enabled: bool,
}

pub fn build_stt_config_message(setup: &SonioxSttSetup) -> String {
    let mut config = json!({
        "api_key": setup.api_key,
        "model": setup.model,
        "audio_format": "pcm_s16le",
        "sample_rate": UPLOAD_SAMPLE_RATE,
        "num_channels": 1,
        "enable_endpoint_detection": true,
        "endpoint_latency_adjustment_level": setup.endpoint_latency_adjustment_level,
        "endpoint_sensitivity": setup.endpoint_sensitivity,
        "max_endpoint_delay_ms": setup.max_endpoint_delay_ms,
    });

    if setup.translation_enabled {
        config["translation"] = json!({
            "type": "one_way",
            "target_language": setup.target_language,
        });
    }

    if !setup.language_hints.is_empty() {
        config["language_hints"] = Value::Array(
            setup
                .language_hints
                .iter()
                .map(|h| Value::String(h.clone()))
                .collect(),
        );
    }

    if let Some(ctx) = build_context_object(&setup.context) {
        config["context"] = ctx;
    }

    config.to_string()
}

/// STT control message — docs: `{"type": "keepalive"}`.
pub fn build_stt_keepalive_message() -> String {
    json!({ "type": "keepalive" }).to_string()
}

pub fn response_finished(value: &Value) -> bool {
    value
        .get("finished")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Parse a Soniox STT server message into zero or more transcript events.
///
/// Tokens with `translation_status: "translation"` feed translated text;
/// `"original"` / `"none"` feed source text. `<end>` finalizes the turn.
pub fn parse_stt_message(
    text: &str,
    direction: &str,
    acc: &mut SonioxTokenAccumulator,
) -> Vec<TranscriptEvent> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };

    if let Some(err) = parse_api_error(&value) {
        tracing::warn!("[Soniox:{direction}] API error: {err}");
        return Vec::new();
    }

    let Some(tokens) = value.get("tokens").and_then(|t| t.as_array()) else {
        return Vec::new();
    };

    // Soniox sends the complete non-final hypothesis in each response. Keep
    // committed finals, but rebuild the live portion from this frame.
    acc.clear_interim();
    let mut events = Vec::new();
    let mut saw_end = false;

    for token in tokens {
        let Some(token_text) = token.get("text").and_then(|t| t.as_str()) else {
            continue;
        };
        if token_text == END_TOKEN {
            saw_end = true;
            continue;
        }
        let status = token
            .get("translation_status")
            .and_then(|s| s.as_str())
            .unwrap_or("none");
        let is_final = token
            .get("is_final")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match status {
            "translation" => {
                if is_final {
                    acc.push_final_translated(token_text);
                } else {
                    acc.push_interim_translated(token_text);
                }
            }
            "original" | "none" => {
                if is_final {
                    acc.push_final_source(token_text);
                } else {
                    acc.push_interim_source(token_text);
                }
            }
            other => {
                tracing::debug!(
                    "[Soniox:{direction}] unknown translation_status={other:?} text={token_text:?}"
                );
            }
        }
    }

    if saw_end {
        // Always emit on <end> (even empty) so TTS fanout can flush pending text.
        events.push(acc.commit_turn(direction));
    } else if let Some(ev) = acc.to_interim_event(direction) {
        events.push(ev);
    }

    events
}

pub fn parse_api_error(value: &Value) -> Option<String> {
    let code = value.get("error_code").or_else(|| value.get("status_code"));
    let msg = value
        .get("error_message")
        .or_else(|| value.get("error"))
        .and_then(|v| v.as_str());
    match (code, msg) {
        (Some(c), Some(m)) => Some(format!("{c}: {m}")),
        (None, Some(m)) => Some(m.to_string()),
        (Some(c), None) => Some(c.to_string()),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct SonioxTokenAccumulator {
    final_source: String,
    final_translated: String,
    interim_source: String,
    interim_translated: String,
}

impl SonioxTokenAccumulator {
    pub fn push_final_source(&mut self, text: &str) {
        self.final_source.push_str(text);
        self.interim_source.clear();
    }

    pub fn push_final_translated(&mut self, text: &str) {
        self.final_translated.push_str(text);
        self.interim_translated.clear();
    }

    pub fn push_interim_source(&mut self, text: &str) {
        self.interim_source.push_str(text);
    }

    pub fn push_interim_translated(&mut self, text: &str) {
        self.interim_translated.push_str(text);
    }

    fn display_source(&self) -> String {
        format!("{}{}", self.final_source, self.interim_source)
    }

    fn display_translated(&self) -> String {
        format!("{}{}", self.final_translated, self.interim_translated)
    }

    pub fn has_content(&self) -> bool {
        !self.display_source().trim().is_empty() || !self.display_translated().trim().is_empty()
    }

    pub fn to_interim_event(&self, direction: &str) -> Option<TranscriptEvent> {
        if !self.has_content() {
            return None;
        }
        Some(TranscriptEvent {
            direction: direction.to_string(),
            source_text: non_empty(&self.display_source()),
            translated_text: non_empty(&self.display_translated()),
            interim: true,
            turn_complete: false,
            input_segment_finished: false,
            output_segment_finished: false,
            connection_gap: false,
            replace_live: true,
            live_source: None,
            live_translated: None,
        })
    }

    /// Finalize the turn on `<end>`. Always returns an event so downstream TTS
    /// can flush even when this frame has no new text.
    pub fn commit_turn(&mut self, direction: &str) -> TranscriptEvent {
        let event = TranscriptEvent {
            direction: direction.to_string(),
            source_text: non_empty(&self.display_source()),
            translated_text: non_empty(&self.display_translated()),
            interim: false,
            turn_complete: true,
            input_segment_finished: true,
            output_segment_finished: true,
            connection_gap: false,
            replace_live: true,
            live_source: None,
            live_translated: None,
        };
        self.clear();
        event
    }

    pub fn clear(&mut self) {
        self.final_source.clear();
        self.final_translated.clear();
        self.clear_interim();
    }

    fn clear_interim(&mut self) {
        self.interim_source.clear();
        self.interim_translated.clear();
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_setup(translation_enabled: bool) -> SonioxSttSetup {
        SonioxSttSetup {
            api_key: "sk-test".into(),
            model: "stt-rt-v5".into(),
            target_language: "en".into(),
            language_hints: vec!["vi".into(), "en".into()],
            context: SonioxContextInput {
                general: vec![crate::providers::shared::live::SonioxGeneralPair {
                    key: "domain".into(),
                    value: "Tech".into(),
                }],
                text: String::new(),
                terms: vec![],
                translation_terms: vec![],
            },
            endpoint_latency_adjustment_level: 2,
            endpoint_sensitivity: 0.3,
            max_endpoint_delay_ms: 1000,
            translation_enabled,
        }
    }

    #[test]
    fn builds_one_way_config() {
        let msg = build_stt_config_message(&sample_setup(true));
        let v: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["translation"]["type"], "one_way");
        assert_eq!(v["translation"]["target_language"], "en");
        assert_eq!(v["sample_rate"], 16000);
        assert_eq!(v["enable_endpoint_detection"], true);
        assert_eq!(v["endpoint_latency_adjustment_level"], 2);
        assert_eq!(v["endpoint_sensitivity"], 0.3);
        assert_eq!(v["max_endpoint_delay_ms"], 1000);
        assert_eq!(v["context"]["general"][0]["value"], "Tech");
    }

    #[test]
    fn builds_stt_only_config_without_translation() {
        let msg = build_stt_config_message(&sample_setup(false));
        let v: Value = serde_json::from_str(&msg).unwrap();
        assert!(v.get("translation").is_none());
        assert_eq!(v["sample_rate"], 16000);
        assert_eq!(v["language_hints"][0], "vi");
    }

    #[test]
    fn stt_keepalive_type_field() {
        let v: Value = serde_json::from_str(&build_stt_keepalive_message()).unwrap();
        assert_eq!(v["type"], "keepalive");
    }

    #[test]
    fn parses_translation_tokens_and_end() {
        let mut acc = SonioxTokenAccumulator::default();
        let interim = r#"{"tokens":[{"text":"Xin","translation_status":"original","is_final":false},{"text":"Hello","translation_status":"translation","is_final":false}]}"#;
        let events = parse_stt_message(interim, "outbound", &mut acc);
        assert_eq!(events.len(), 1);
        assert!(events[0].interim);
        assert_eq!(events[0].translated_text.as_deref(), Some("Hello"));

        let fin = r#"{"tokens":[{"text":"Xin","translation_status":"original","is_final":true},{"text":"Hello","translation_status":"translation","is_final":true},{"text":"<end>","is_final":true}]}"#;
        let events = parse_stt_message(fin, "outbound", &mut acc);
        assert_eq!(events.len(), 1);
        assert!(events[0].turn_complete);
        assert!(!events[0].interim);
    }

    #[test]
    fn non_final_hypotheses_replace_each_frame() {
        let mut acc = SonioxTokenAccumulator::default();
        let events = parse_stt_message(
            r#"{"tokens":[{"text":"How","is_final":false},{"text":"'re","is_final":false}]}"#,
            "inbound",
            &mut acc,
        );
        assert_eq!(events[0].source_text.as_deref(), Some("How're"));

        let events = parse_stt_message(
            r#"{"tokens":[{"text":"How","is_final":false},{"text":" ","is_final":false},{"text":"are","is_final":false}]}"#,
            "inbound",
            &mut acc,
        );
        assert_eq!(events[0].source_text.as_deref(), Some("How are"));
    }

    #[test]
    fn finals_persist_while_non_finals_replace() {
        let mut acc = SonioxTokenAccumulator::default();
        let events = parse_stt_message(
            r#"{"tokens":[{"text":"How","is_final":true},{"text":" ","is_final":true},{"text":"are","is_final":false},{"text":" you","is_final":false}]}"#,
            "outbound",
            &mut acc,
        );
        assert_eq!(events[0].source_text.as_deref(), Some("How are you"));
    }

    #[test]
    fn original_and_translation_hypotheses_are_replaced_independently() {
        let mut acc = SonioxTokenAccumulator::default();
        let _ = parse_stt_message(
            r#"{"tokens":[{"text":"Xin","translation_status":"original","is_final":false},{"text":"Hello","translation_status":"translation","is_final":false}]}"#,
            "outbound",
            &mut acc,
        );
        let events = parse_stt_message(
            r#"{"tokens":[{"text":"Chào","translation_status":"original","is_final":false},{"text":"Hi","translation_status":"translation","is_final":false}]}"#,
            "outbound",
            &mut acc,
        );
        assert_eq!(events[0].source_text.as_deref(), Some("Chào"));
        assert_eq!(events[0].translated_text.as_deref(), Some("Hi"));
    }

    #[test]
    fn empty_end_still_emits_turn_complete() {
        let mut acc = SonioxTokenAccumulator::default();
        let events = parse_stt_message(
            r#"{"tokens":[{"text":"<end>","is_final":true}]}"#,
            "outbound",
            &mut acc,
        );
        assert_eq!(events.len(), 1);
        assert!(events[0].turn_complete);
        assert!(events[0].translated_text.is_none());
    }

    #[test]
    fn api_error_yields_no_events() {
        let mut acc = SonioxTokenAccumulator::default();
        let events = parse_stt_message(
            r#"{"error_code":401,"error_message":"bad key"}"#,
            "outbound",
            &mut acc,
        );
        assert!(events.is_empty());
    }

    #[test]
    fn non_empty_trims() {
        assert_eq!(non_empty("  hi  ").as_deref(), Some("hi"));
        assert!(non_empty("   ").is_none());
    }
}
