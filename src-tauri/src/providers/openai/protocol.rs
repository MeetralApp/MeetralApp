use std::time::{Duration, Instant};

use base64::Engine;
use serde_json::Value;

use crate::providers::openai::config::{OPENAI_TRANSCRIPTION_MODEL, UPLOAD_SAMPLE_RATE};
use crate::providers::shared::live::TranscriptEvent;

/// Slightly longer than Gemini VAD default — OpenAI streams lack `turnComplete`.
pub const PHRASE_SILENCE_COMMIT_MS: u64 = 1500;

/// Minimum idle time after both transcript `.done` events before committing (debounce).
const PAIRED_DONE_DEBOUNCE_MS: u64 = 200;

#[derive(Debug, Clone)]
pub struct OpenAiAudioChunk {
    pub pcm_24k: Vec<i16>,
}

#[derive(Debug, Default)]
pub struct TranscriptAccumulator {
    pub source: String,
    pub translated: String,
}

impl TranscriptAccumulator {
    pub fn apply_delta(&mut self, source_delta: Option<&str>, translated_delta: Option<&str>) {
        if let Some(delta) = source_delta {
            self.source.push_str(delta);
        }
        if let Some(delta) = translated_delta {
            self.translated.push_str(delta);
        }
    }

    pub fn has_content(&self) -> bool {
        !self.source.trim().is_empty() || !self.translated.trim().is_empty()
    }

    pub fn clear(&mut self) {
        self.source.clear();
        self.translated.clear();
    }

    pub fn to_interim_event(&self, direction: &str) -> Option<TranscriptEvent> {
        if !self.has_content() {
            return None;
        }
        Some(TranscriptEvent {
            direction: direction.to_string(),
            source_text: non_empty(&self.source),
            translated_text: non_empty(&self.translated),
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
}

/// Accumulates OpenAI transcript deltas and emits synthetic `turn_complete` events
/// so SegmentEngine can commit phrases like Gemini's `turnComplete`.
///
/// Commit policy (conservative — avoids punctuation / micro-fragment splits):
/// 1. Both `input_transcript.done` and `output_transcript.done` received, then debounced.
/// 2. Silence fallback after [`PHRASE_SILENCE_COMMIT_MS`] with substantial content.
/// 3. Session close flush (keeps any non-junk remainder).
#[derive(Debug, Default)]
pub struct PhraseCommitter {
    acc: TranscriptAccumulator,
    last_transcript_at: Option<Instant>,
    input_done_pending: bool,
    output_done_pending: bool,
    paired_done_at: Option<Instant>,
}

impl PhraseCommitter {
    pub fn check_silence(&mut self, direction: &str) -> Vec<TranscriptEvent> {
        let Some(last) = self.last_transcript_at else {
            return Vec::new();
        };
        if last.elapsed() < Duration::from_millis(PHRASE_SILENCE_COMMIT_MS) {
            return Vec::new();
        }
        if !self.acc.has_content() {
            return Vec::new();
        }
        if is_junk_fragment(&self.acc.source, &self.acc.translated) {
            self.trim_junk_prefixes();
            self.last_transcript_at = None;
            return Vec::new();
        }
        self.commit_all(direction)
    }

    pub fn check_paired_done(&mut self, direction: &str) -> Vec<TranscriptEvent> {
        if !(self.input_done_pending && self.output_done_pending) {
            return Vec::new();
        }
        let Some(paired_at) = self.paired_done_at else {
            return Vec::new();
        };
        if paired_at.elapsed() < Duration::from_millis(PAIRED_DONE_DEBOUNCE_MS) {
            return Vec::new();
        }
        if !self.acc.has_content() {
            self.clear_done_flags();
            return Vec::new();
        }
        if is_junk_fragment(&self.acc.source, &self.acc.translated) {
            self.trim_junk_prefixes();
            self.clear_done_flags();
            self.last_transcript_at = Some(Instant::now());
            return Vec::new();
        }
        self.commit_all(direction)
    }

    pub fn flush(&mut self, direction: &str) -> Vec<TranscriptEvent> {
        if !self.acc.has_content() {
            return Vec::new();
        }
        if is_junk_fragment(&self.acc.source, &self.acc.translated) {
            self.trim_junk_prefixes();
            if !self.acc.has_content() {
                return Vec::new();
            }
        }
        self.commit_all(direction)
    }

    fn touch_transcript(&mut self) {
        self.last_transcript_at = Some(Instant::now());
    }

    fn clear_done_flags(&mut self) {
        self.input_done_pending = false;
        self.output_done_pending = false;
        self.paired_done_at = None;
    }

    fn mark_input_done(&mut self) {
        self.input_done_pending = true;
        self.maybe_mark_paired();
    }

    fn mark_output_done(&mut self) {
        self.output_done_pending = true;
        self.maybe_mark_paired();
    }

    fn maybe_mark_paired(&mut self) {
        if self.input_done_pending && self.output_done_pending {
            self.paired_done_at.get_or_insert_with(Instant::now);
        }
    }

    fn trim_junk_prefixes(&mut self) {
        self.acc.source = trim_leading_junk(&self.acc.source);
        self.acc.translated = trim_leading_junk(&self.acc.translated);
    }

    fn commit_all(&mut self, direction: &str) -> Vec<TranscriptEvent> {
        let source = std::mem::take(&mut self.acc.source);
        let translated = std::mem::take(&mut self.acc.translated);
        self.last_transcript_at = None;
        self.clear_done_flags();
        vec![turn_complete_event(direction, source, translated)]
    }

    fn after_transcript_delta(&mut self, direction: &str) -> Vec<TranscriptEvent> {
        self.touch_transcript();
        if let Some(interim) = self.acc.to_interim_event(direction) {
            vec![interim]
        } else {
            Vec::new()
        }
    }

    pub fn on_input_delta(&mut self, direction: &str, delta: &str) -> Vec<TranscriptEvent> {
        self.input_done_pending = false;
        self.paired_done_at = None;
        self.acc.apply_delta(Some(delta), None);
        self.after_transcript_delta(direction)
    }

    pub fn on_output_delta(&mut self, direction: &str, delta: &str) -> Vec<TranscriptEvent> {
        self.output_done_pending = false;
        self.paired_done_at = None;
        self.acc.apply_delta(None, Some(delta));
        self.after_transcript_delta(direction)
    }

    pub fn on_input_done(&mut self, direction: &str, transcript: &str) -> Vec<TranscriptEvent> {
        if !transcript.trim().is_empty() {
            self.acc.source = transcript.to_string();
        }
        self.mark_input_done();
        self.touch_transcript();
        let mut events = self.after_transcript_delta(direction);
        events.extend(self.check_paired_done(direction));
        events
    }

    pub fn on_output_done(&mut self, direction: &str, transcript: &str) -> Vec<TranscriptEvent> {
        if !transcript.trim().is_empty() {
            self.acc.translated = transcript.to_string();
        }
        self.mark_output_done();
        self.touch_transcript();
        let mut events = self.after_transcript_delta(direction);
        events.extend(self.check_paired_done(direction));
        events
    }

    /// Notes / Realtime transcription: source-only deltas (display as both columns).
    pub fn on_transcription_delta(&mut self, direction: &str, delta: &str) -> Vec<TranscriptEvent> {
        self.acc.apply_delta(Some(delta), Some(delta));
        self.after_transcript_delta(direction)
    }

    /// Notes: final transcript for a committed audio item — commit immediately.
    pub fn on_transcription_completed(
        &mut self,
        direction: &str,
        transcript: &str,
    ) -> Vec<TranscriptEvent> {
        let text = transcript.trim();
        if text.is_empty() {
            return Vec::new();
        }
        if is_junk_fragment(text, text) {
            self.acc.clear();
            self.clear_done_flags();
            return Vec::new();
        }
        self.acc.source = text.to_string();
        self.acc.translated = text.to_string();
        self.commit_all(direction)
    }
}

fn non_empty(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn turn_complete_event(direction: &str, source: String, translated: String) -> TranscriptEvent {
    TranscriptEvent {
        direction: direction.to_string(),
        source_text: non_empty(&source),
        translated_text: non_empty(&translated),
        interim: false,
        turn_complete: true,
        input_segment_finished: true,
        output_segment_finished: true,
        connection_gap: false,
        replace_live: true,
        live_source: None,
        live_translated: None,
    }
}

fn meaningful_word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(|c| c.is_alphanumeric()))
        .count()
}

fn is_punctuation_only(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && !trimmed.chars().any(|c| c.is_alphanumeric())
}

/// Rejects micro-fragments like `".."` / `"this"` or `"."` / `"ID"` from becoming segments.
fn is_junk_fragment(source: &str, translated: &str) -> bool {
    let s = source.trim();
    let t = translated.trim();

    if s.is_empty() && t.is_empty() {
        return true;
    }

    let s_words = meaningful_word_count(s);
    let t_words = meaningful_word_count(t);

    if s_words == 0 && t_words == 0 {
        return true;
    }

    if is_punctuation_only(s) && t_words <= 1 && t.len() < 8 {
        return true;
    }
    if is_punctuation_only(t) && s_words <= 1 && s.len() < 8 {
        return true;
    }

    let total_words = s_words + t_words;
    let max_chars = s.chars().count().max(t.chars().count());
    if total_words < 2 && max_chars < 10 {
        return true;
    }

    false
}

fn trim_leading_junk(text: &str) -> String {
    let trimmed = text.trim_start();
    let rest = trimmed
        .trim_start_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .trim_start();
    rest.to_string()
}

pub fn build_session_update(target_language: &str, noise_reduction: &str) -> String {
    serde_json::json!({
        "type": "session.update",
        "session": {
            "audio": {
                "input": {
                    "transcription": { "model": OPENAI_TRANSCRIPTION_MODEL },
                    "noise_reduction": { "type": noise_reduction }
                },
                "output": { "language": target_language }
            }
        }
    })
    .to_string()
}

/// Notes: Realtime transcription session (no translation / no spoken response).
pub fn build_transcription_session_update(language: &str, noise_reduction: &str) -> String {
    serde_json::json!({
        "type": "session.update",
        "session": {
            "type": "transcription",
            "audio": {
                "input": {
                    "format": {
                        "type": "audio/pcm",
                        "rate": UPLOAD_SAMPLE_RATE
                    },
                    "transcription": {
                        "model": OPENAI_TRANSCRIPTION_MODEL,
                        "language": language,
                        "delay": "low"
                    },
                    "noise_reduction": { "type": noise_reduction },
                    "turn_detection": null
                }
            }
        }
    })
    .to_string()
}

pub fn build_audio_append(pcm: &[i16]) -> String {
    let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
    let audio = base64::engine::general_purpose::STANDARD.encode(bytes);
    serde_json::json!({
        "type": "session.input_audio_buffer.append",
        "audio": audio
    })
    .to_string()
}

/// Notes transcription sessions use the non-`session.` event name.
pub fn build_transcription_audio_append(pcm: &[i16]) -> String {
    let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
    let audio = base64::engine::general_purpose::STANDARD.encode(bytes);
    serde_json::json!({
        "type": "input_audio_buffer.append",
        "audio": audio
    })
    .to_string()
}

pub fn build_transcription_audio_commit() -> String {
    serde_json::json!({ "type": "input_audio_buffer.commit" }).to_string()
}

pub fn build_session_close() -> String {
    serde_json::json!({ "type": "session.close" }).to_string()
}

pub fn decode_pcm16_base64(data: &str) -> Option<Vec<i16>> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    if bytes.len() < 2 {
        return None;
    }
    let samples: Vec<i16> = bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    if samples.is_empty() {
        None
    } else {
        Some(samples)
    }
}

pub fn parse_api_error(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(|e| {
            e.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
                .or_else(|| Some(e.to_string()))
        })
        .or_else(|| {
            value
                .pointer("/error/message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
}

pub fn parse_server_event(
    value: &Value,
    direction: &str,
    committer: &mut PhraseCommitter,
) -> (bool, Vec<OpenAiAudioChunk>, Vec<TranscriptEvent>, bool) {
    let event_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let mut setup_complete = false;
    let mut session_closed = false;
    let mut audio_chunks = Vec::new();
    let mut transcripts = Vec::new();

    match event_type {
        "session.created" | "session.updated" => {
            setup_complete = true;
        }
        "session.output_audio.delta" => {
            if let Some(delta) = value.get("delta").and_then(|d| d.as_str()) {
                if let Some(samples) = decode_pcm16_base64(delta) {
                    audio_chunks.push(OpenAiAudioChunk { pcm_24k: samples });
                }
            }
        }
        "session.input_transcript.delta" => {
            if let Some(delta) = value.get("delta").and_then(|d| d.as_str()) {
                transcripts.extend(committer.on_input_delta(direction, delta));
            }
        }
        "session.output_transcript.delta" => {
            if let Some(delta) = value.get("delta").and_then(|d| d.as_str()) {
                transcripts.extend(committer.on_output_delta(direction, delta));
            }
        }
        "session.input_transcript.done" => {
            if let Some(transcript) = value.get("transcript").and_then(|t| t.as_str()) {
                transcripts.extend(committer.on_input_done(direction, transcript));
            }
        }
        "session.output_transcript.done" => {
            if let Some(transcript) = value.get("transcript").and_then(|t| t.as_str()) {
                transcripts.extend(committer.on_output_done(direction, transcript));
            }
        }
        "conversation.item.input_audio_transcription.delta" => {
            if let Some(delta) = value.get("delta").and_then(|d| d.as_str()) {
                transcripts.extend(committer.on_transcription_delta(direction, delta));
            }
        }
        "conversation.item.input_audio_transcription.completed" => {
            if let Some(transcript) = value.get("transcript").and_then(|t| t.as_str()) {
                transcripts.extend(committer.on_transcription_completed(direction, transcript));
            }
        }
        "session.closed" => {
            session_closed = true;
            transcripts.extend(committer.flush(direction));
        }
        "error" | "session.error" => {}
        _ => {}
    }

    (setup_complete, audio_chunks, transcripts, session_closed)
}

pub fn upload_sample_rate() -> u32 {
    UPLOAD_SAMPLE_RATE
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_transcription_session_has_no_output_language() {
        let msg = build_transcription_session_update("vi", "near_field");
        let value: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            value.pointer("/session/type").and_then(|v| v.as_str()),
            Some("transcription")
        );
        assert!(value.pointer("/session/audio/output").is_none());
        assert_eq!(
            value
                .pointer("/session/audio/input/transcription/language")
                .and_then(|v| v.as_str()),
            Some("vi")
        );
    }

    #[test]
    fn transcription_completed_commits_source_only() {
        let mut committer = PhraseCommitter::default();
        let delta = json!({
            "type": "conversation.item.input_audio_transcription.delta",
            "delta": "Xin "
        });
        let (_, _, interim, _) = parse_server_event(&delta, "outbound", &mut committer);
        assert!(interim.iter().any(|e| e.interim));

        let done = json!({
            "type": "conversation.item.input_audio_transcription.completed",
            "transcript": "Xin chào mọi người."
        });
        let (_, _, events, _) = parse_server_event(&done, "outbound", &mut committer);
        let complete = events.iter().find(|e| e.turn_complete).unwrap();
        assert_eq!(complete.source_text.as_deref(), Some("Xin chào mọi người."));
        assert_eq!(
            complete.translated_text.as_deref(),
            Some("Xin chào mọi người.")
        );
    }

    #[test]
    fn build_session_update_contains_language() {
        let msg = build_session_update("vi", "near_field");
        let value: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            value
                .pointer("/session/audio/output/language")
                .and_then(|v| v.as_str()),
            Some("vi")
        );
        assert_eq!(
            value
                .pointer("/session/audio/input/transcription/model")
                .and_then(|v| v.as_str()),
            Some("gpt-realtime-whisper")
        );
    }

    #[test]
    fn parse_output_audio_delta() {
        let samples = vec![1000i16, -500i16];
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        let value = json!({
            "type": "session.output_audio.delta",
            "delta": encoded
        });
        let mut committer = PhraseCommitter::default();
        let (_, chunks, transcripts, _) = parse_server_event(&value, "outbound", &mut committer);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].pcm_24k, samples);
        assert!(transcripts.is_empty());
    }

    #[test]
    fn parse_transcript_deltas_accumulate_interim() {
        let mut committer = PhraseCommitter::default();
        let input = json!({"type": "session.input_transcript.delta", "delta": "Hello"});
        let output = json!({"type": "session.output_transcript.delta", "delta": "Xin chào"});
        let (_, _, t1, _) = parse_server_event(&input, "inbound", &mut committer);
        let (_, _, t2, _) = parse_server_event(&output, "inbound", &mut committer);
        assert_eq!(t1.len(), 1);
        assert!(t1[0].interim);
        assert!(!t1[0].turn_complete);
        assert_eq!(t1[0].source_text.as_deref(), Some("Hello"));
        assert_eq!(t2[0].translated_text.as_deref(), Some("Xin chào"));
    }

    #[test]
    fn defers_commit_until_both_done_events() {
        let mut committer = PhraseCommitter::default();
        let output = json!({"type": "session.output_transcript.delta", "delta": "Xin chào."});
        let (_, _, events, _) = parse_server_event(&output, "outbound", &mut committer);
        assert!(events.iter().all(|e| !e.turn_complete));

        let input_done = json!({
            "type": "session.input_transcript.done",
            "transcript": "Hello."
        });
        let (_, _, events, _) = parse_server_event(&input_done, "outbound", &mut committer);
        assert!(events.iter().all(|e| !e.turn_complete));

        let output_done = json!({
            "type": "session.output_transcript.done",
            "transcript": "Xin chào."
        });
        let (_, _, events, _) = parse_server_event(&output_done, "outbound", &mut committer);
        assert!(
            events.iter().all(|e| !e.turn_complete),
            "debounce before commit"
        );

        committer.paired_done_at =
            Some(Instant::now() - Duration::from_millis(PAIRED_DONE_DEBOUNCE_MS + 50));
        let events = committer.check_paired_done("outbound");
        assert!(events.iter().any(|e| e.turn_complete));
        let complete = events.iter().find(|e| e.turn_complete).unwrap();
        assert_eq!(complete.source_text.as_deref(), Some("Hello."));
        assert_eq!(complete.translated_text.as_deref(), Some("Xin chào."));
    }

    #[test]
    fn is_junk_fragment_detects_punctuation_noise() {
        assert!(is_junk_fragment("..", "this"));
        assert!(is_junk_fragment(".", "ID"));
        assert!(is_junk_fragment("", ""));
        assert!(!is_junk_fragment(
            "ý tưởng này có đi xuyên suốt các dự án hay không.",
            "goes across the projects or not"
        ));
    }

    #[test]
    fn rejects_junk_fragment_commit() {
        let mut committer = PhraseCommitter::default();
        committer.on_input_delta("outbound", "..");
        committer.on_output_delta("outbound", "this");
        committer.last_transcript_at =
            Some(Instant::now() - Duration::from_millis(PHRASE_SILENCE_COMMIT_MS + 50));
        let events = committer.check_silence("outbound");
        assert!(events.is_empty(), "junk fragments must not commit");
        assert!(committer.acc.has_content() || !committer.acc.source.is_empty());
    }

    #[test]
    fn silence_commit_after_pause() {
        let mut committer = PhraseCommitter::default();
        committer.on_input_delta("outbound", "Still talking here");
        committer.on_output_delta("outbound", "Vẫn đang nói");
        committer.last_transcript_at =
            Some(Instant::now() - Duration::from_millis(PHRASE_SILENCE_COMMIT_MS + 50));
        let events = committer.check_silence("outbound");
        assert_eq!(events.len(), 1);
        assert!(events[0].turn_complete);
        assert_eq!(events[0].source_text.as_deref(), Some("Still talking here"));
    }

    #[test]
    fn flush_on_session_closed() {
        let mut committer = PhraseCommitter::default();
        committer.on_output_delta("inbound", "Partial");
        let value = json!({"type": "session.closed"});
        let (_, _, events, closed) = parse_server_event(&value, "inbound", &mut committer);
        assert!(closed);
        assert_eq!(events.len(), 1);
        assert!(events[0].turn_complete);
        assert_eq!(events[0].translated_text.as_deref(), Some("Partial"));
    }

    #[test]
    fn session_created_marks_setup() {
        let value = json!({"type": "session.created"});
        let mut committer = PhraseCommitter::default();
        let (setup, _, _, _) = parse_server_event(&value, "outbound", &mut committer);
        assert!(setup);
    }

    #[test]
    fn turn_complete_commits_via_segment_engine() {
        use crate::meeting::segment_engine::{CommitReason, SegmentEngine};
        use std::collections::HashMap;

        let mut committer = PhraseCommitter::default();
        committer.on_input_delta("outbound", "Hello.");
        let events = committer.on_output_delta("outbound", "Xin chào.");
        assert!(events.iter().all(|e| !e.turn_complete));

        committer.on_input_done("outbound", "Hello.");
        let events = committer.on_output_done("outbound", "Xin chào.");
        assert!(events.iter().all(|e| !e.turn_complete));

        committer.paired_done_at =
            Some(Instant::now() - Duration::from_millis(PAIRED_DONE_DEBOUNCE_MS + 50));
        let events = committer.check_paired_done("outbound");
        let complete = events
            .iter()
            .find(|e| e.turn_complete)
            .expect("turn_complete event");

        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );
        let result = engine.process(complete, true);
        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.commits[0].reason, CommitReason::Turn);
        assert_eq!(result.commits[0].translated, "Xin chào.");
    }
}
