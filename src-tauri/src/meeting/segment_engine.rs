//! Authoritative transcript segment boundary logic (single source of truth).
//! Frontend consumes `segment-preview` + enriched `transcript` events — no duplicate split rules.

use std::collections::HashMap;

use serde::Serialize;

use crate::ai::TranscriptEvent;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UtteranceBlock {
    pub source: String,
    pub translated: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitReason {
    Sentence,
    Turn,
    Gap,
    /// Forced commit on stop translate / end meeting (trailing live text).
    Flush,
}

#[derive(Debug, Clone)]
pub struct SegmentCommit {
    pub direction: String,
    pub sequence: i32,
    pub source: String,
    pub translated: String,
    pub connection_gap: bool,
    pub reason: CommitReason,
    /// Process mono ms when live utterance opened; `None` for instantaneous gap markers.
    pub opened_at_mono: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveSnapshot {
    pub source: Option<String>,
    pub translated: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessResult {
    pub live: LiveSnapshot,
    pub commits: Vec<SegmentCommit>,
}

/// Identical re-delivery of the just-committed utterance within this window
/// (overlapping sessions at start, reconnect replay) is dropped. Genuine
/// repeats of the same sentence take longer than this in practice.
const DUPLICATE_COMMIT_WINDOW_MS: i64 = 3_000;

/// Fingerprint of the most recent real commit — idempotency reference.
struct LastCommit {
    source: String,
    translated: String,
    committed_at_mono: i64,
}

struct DirectionState {
    live: Option<UtteranceBlock>,
    /// Set when `live` first becomes non-empty / allocated.
    live_opened_at_mono: Option<i64>,
    next_sequence: i32,
    /// After stop/end flush, ignore late relay events until translate starts again.
    accepting: bool,
    /// Last committed utterance text — gap markers do not overwrite this.
    last_commit: Option<LastCommit>,
}

impl Default for DirectionState {
    fn default() -> Self {
        Self {
            live: None,
            live_opened_at_mono: None,
            next_sequence: 1,
            accepting: true,
            last_commit: None,
        }
    }
}

pub struct SegmentEngine {
    meeting_id: Option<String>,
    /// Wall `meeting_record.started_at_ms` — retained for prepare identity; timing uses mono open stamps.
    #[allow(dead_code)]
    meeting_started_ms: i64,
    directions: HashMap<String, DirectionState>,
}

impl SegmentEngine {
    pub fn new(meeting_started_ms: i64) -> Self {
        Self {
            meeting_id: None,
            meeting_started_ms,
            directions: HashMap::new(),
        }
    }

    pub fn meeting_id(&self) -> Option<&str> {
        self.meeting_id.as_deref()
    }

    pub fn reset_for_meeting(
        &mut self,
        meeting_id: Option<String>,
        meeting_started_ms: i64,
        next_sequences: HashMap<String, i32>,
    ) {
        self.meeting_id = meeting_id;
        self.meeting_started_ms = meeting_started_ms;
        self.directions.clear();
        for (direction, next) in next_sequences {
            self.directions.insert(
                direction,
                DirectionState {
                    live: None,
                    live_opened_at_mono: None,
                    next_sequence: next.max(1),
                    accepting: true,
                    last_commit: None,
                },
            );
        }
    }

    pub fn process(&mut self, event: &TranscriptEvent, persist: bool) -> ProcessResult {
        if event.interim && !event.turn_complete && !event.connection_gap {
            let has_text = event
                .source_text
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
                || event
                    .translated_text
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty());
            if !has_text {
                return ProcessResult::default();
            }
        }

        // Sealed after flush — drop late turn_complete / interim that would re-open live.
        if !self.is_accepting(&event.direction) {
            return ProcessResult::default();
        }

        let mut commits = Vec::new();

        if event.connection_gap {
            if persist {
                if let Some(commit) = self.commit_direction(&event.direction, CommitReason::Gap) {
                    commits.push(commit);
                }
            } else {
                self.clear_live(&event.direction);
            }
            if persist {
                let source = event
                    .source_text
                    .as_deref()
                    .unwrap_or("[Connection interrupted]");
                let translated = event.translated_text.as_deref().unwrap_or(source);
                if let Some(commit) = self.push_gap_commit(
                    &event.direction,
                    source.to_string(),
                    translated.to_string(),
                ) {
                    commits.push(commit);
                }
            }
            return ProcessResult {
                live: LiveSnapshot::default(),
                commits,
            };
        }

        if event.turn_complete {
            let state = self.directions.entry(event.direction.clone()).or_default();
            Self::ensure_live(state);
            if let Some(live) = state.live.as_mut() {
                apply_text(
                    live,
                    event.source_text.as_deref(),
                    event.translated_text.as_deref(),
                    event.replace_live,
                );
            }
            if persist {
                if let Some(commit) = self.commit_direction(&event.direction, CommitReason::Turn) {
                    commits.push(commit);
                }
            } else {
                self.clear_live(&event.direction);
            }
            return ProcessResult {
                live: self.live_snapshot(&event.direction),
                commits,
            };
        }

        let has_source = event
            .source_text
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
        let has_translated = event
            .translated_text
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
        if !has_source && !has_translated {
            return ProcessResult::default();
        }

        let should_commit = self
            .directions
            .get(&event.direction)
            .and_then(|state| state.live.as_ref())
            .and_then(|live| {
                if !has_source {
                    return None;
                }
                let src = event.source_text.as_deref()?;
                if is_distinct_utterance(&live.source, src)
                    && (ends_sentence(Some(&live.source)) || ends_sentence(Some(&live.translated)))
                {
                    Some(())
                } else {
                    None
                }
            })
            .is_some();

        if should_commit && persist {
            if let Some(commit) = self.commit_direction(&event.direction, CommitReason::Sentence) {
                commits.push(commit);
            }
        }

        let state = self.directions.entry(event.direction.clone()).or_default();

        Self::ensure_live(state);
        if let Some(live) = state.live.as_mut() {
            apply_text(
                live,
                event.source_text.as_deref(),
                event.translated_text.as_deref(),
                event.replace_live,
            );
        }

        ProcessResult {
            live: self.live_snapshot(&event.direction),
            commits,
        }
    }

    fn ensure_live(state: &mut DirectionState) {
        if state.live.is_none() {
            state.live = Some(UtteranceBlock::default());
            state.live_opened_at_mono = Some(crate::audio::monotonic_ms() as i64);
        }
    }

    fn live_snapshot(&self, direction: &str) -> LiveSnapshot {
        self.directions
            .get(direction)
            .and_then(|s| s.live.as_ref())
            .map(|live| {
                let source = live.source.trim();
                let translated = live.translated.trim();
                LiveSnapshot {
                    source: (!source.is_empty()).then(|| source.to_string()),
                    translated: (!translated.is_empty()).then(|| translated.to_string()),
                }
            })
            .unwrap_or_default()
    }

    fn clear_live(&mut self, direction: &str) {
        if let Some(state) = self.directions.get_mut(direction) {
            state.live = None;
            state.live_opened_at_mono = None;
        }
    }

    fn commit_direction(&mut self, direction: &str, reason: CommitReason) -> Option<SegmentCommit> {
        let state = self.directions.get_mut(direction)?;
        let block = state.live.take()?;
        let opened_at_mono = state.live_opened_at_mono.take();
        if block.source.trim().is_empty() && block.translated.trim().is_empty() {
            return None;
        }
        let source = block.source;
        // Notes / STT-only: keep FE bilingual columns filled when MT is absent.
        let translated = if block.translated.trim().is_empty() {
            source.clone()
        } else {
            block.translated
        };
        let now_mono = crate::audio::monotonic_ms() as i64;
        if let Some(last) = &state.last_commit {
            let re_delivered = last.source == source.trim()
                && last.translated == translated.trim()
                && now_mono - last.committed_at_mono <= DUPLICATE_COMMIT_WINDOW_MS;
            if re_delivered {
                // Overlapping session / reconnect replay delivered the same
                // utterance twice. `live` was already taken above, so the
                // replayed text is dropped here — no row, no sequence consumed.
                tracing::info!(
                    direction = %direction,
                    "dropping duplicate segment commit (re-delivered utterance)"
                );
                return None;
            }
        }
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        state.last_commit = Some(LastCommit {
            source: source.trim().to_string(),
            translated: translated.trim().to_string(),
            committed_at_mono: now_mono,
        });
        Some(SegmentCommit {
            direction: direction.to_string(),
            sequence,
            source,
            translated,
            connection_gap: false,
            reason,
            opened_at_mono,
        })
    }

    fn push_gap_commit(
        &mut self,
        direction: &str,
        source: String,
        translated: String,
    ) -> Option<SegmentCommit> {
        let state = self.directions.entry(direction.to_string()).or_default();
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        Some(SegmentCommit {
            direction: direction.to_string(),
            sequence,
            source,
            translated,
            connection_gap: true,
            reason: CommitReason::Gap,
            opened_at_mono: None,
        })
    }

    fn is_accepting(&self, direction: &str) -> bool {
        self.directions
            .get(direction)
            .map(|s| s.accepting)
            .unwrap_or(true)
    }

    fn seal_direction(&mut self, direction: &str) {
        self.directions
            .entry(direction.to_string())
            .or_default()
            .accepting = false;
    }

    /// Re-open a direction after stop flush so a new translate session can commit again.
    pub fn open_direction(&mut self, direction: &str) {
        self.directions
            .entry(direction.to_string())
            .or_default()
            .accepting = true;
    }

    /// Commit any non-empty live buffer for one direction (stop / end meeting).
    /// Seals the direction so late relay events cannot duplicate the flushed text.
    pub fn flush_direction(&mut self, direction: &str) -> Option<SegmentCommit> {
        let commit = self.commit_direction(direction, CommitReason::Flush);
        self.seal_direction(direction);
        commit
    }

    /// Flush outbound then inbound live buffers.
    pub fn flush_all(&mut self) -> Vec<SegmentCommit> {
        let mut commits = Vec::with_capacity(2);
        if let Some(c) = self.flush_direction("outbound") {
            commits.push(c);
        }
        if let Some(c) = self.flush_direction("inbound") {
            commits.push(c);
        }
        commits
    }

    /// Ages every direction's last-commit fingerprint past the duplicate
    /// window — simulates time passing without sleeping in tests.
    #[cfg(test)]
    fn debug_age_last_commit_for_test(&mut self) {
        for state in self.directions.values_mut() {
            if let Some(last) = &mut state.last_commit {
                last.committed_at_mono -= DUPLICATE_COMMIT_WINDOW_MS + 1;
            }
        }
    }
}

pub struct SharedSegmentEngine {
    inner: std::sync::Mutex<SegmentEngine>,
}

impl SharedSegmentEngine {
    pub fn new(meeting_started_ms: i64) -> Self {
        Self {
            inner: std::sync::Mutex::new(SegmentEngine::new(meeting_started_ms)),
        }
    }

    pub fn prepare_meeting(
        &self,
        meeting_id: &str,
        meeting_started_ms: i64,
        outbound_next: i32,
        inbound_next: i32,
    ) {
        let mut engine = crate::meeting::lock_poison_recover(&self.inner, "segment engine");
        if engine.meeting_id() == Some(meeting_id) {
            return;
        }
        let mut next = HashMap::new();
        next.insert("outbound".to_string(), outbound_next.max(1));
        next.insert("inbound".to_string(), inbound_next.max(1));
        engine.reset_for_meeting(Some(meeting_id.to_string()), meeting_started_ms, next);
    }

    pub fn clear_meeting(&self) {
        let mut engine = crate::meeting::lock_poison_recover(&self.inner, "segment engine");
        engine.reset_for_meeting(None, 0, HashMap::new());
    }

    pub fn process(&self, event: &TranscriptEvent, persist: bool) -> ProcessResult {
        crate::meeting::lock_poison_recover(&self.inner, "segment engine").process(event, persist)
    }

    pub fn flush_direction(&self, direction: &str) -> Option<SegmentCommit> {
        crate::meeting::lock_poison_recover(&self.inner, "segment engine")
            .flush_direction(direction)
    }

    pub fn flush_all(&self) -> Vec<SegmentCommit> {
        crate::meeting::lock_poison_recover(&self.inner, "segment engine").flush_all()
    }

    pub fn open_direction(&self, direction: &str) {
        crate::meeting::lock_poison_recover(&self.inner, "segment engine").open_direction(direction)
    }
}

fn merge_streaming_text(previous: &str, next: &str) -> String {
    if next.trim().is_empty() {
        return previous.to_string();
    }
    if previous.trim().is_empty() {
        return next.trim().to_string();
    }
    let prev = previous.trim();
    let next_trimmed = next.trim();
    if next_trimmed.starts_with(prev) {
        return next_trimmed.to_string();
    }
    if prev.starts_with(next_trimmed) {
        return prev.to_string();
    }
    let needs_space = !prev.ends_with([' ', '-', '(', ','])
        && !prev.ends_with('—')
        && !next_trimmed.starts_with(|c: char| ",.;:!?)".contains(c));
    if needs_space {
        format!("{prev} {next_trimmed}")
    } else {
        format!("{prev}{next_trimmed}")
    }
}

fn apply_text(
    block: &mut UtteranceBlock,
    source: Option<&str>,
    translated: Option<&str>,
    replace_live: bool,
) {
    if let Some(s) = source.filter(|t| !t.trim().is_empty()) {
        block.source = if replace_live {
            s.trim().to_string()
        } else {
            merge_streaming_text(&block.source, s)
        };
    }
    if let Some(t) = translated.filter(|x| !x.trim().is_empty()) {
        block.translated = if replace_live {
            t.trim().to_string()
        } else {
            merge_streaming_text(&block.translated, t)
        };
    }
}

/// Canonical sentence-end detector — same rules as streaming TTS
/// (`streaming_text_ends_sentence`), including CJK terminators.
fn ends_sentence(text: Option<&str>) -> bool {
    text.is_some_and(crate::voice::shared::tts_text::streaming_text_ends_sentence)
}

fn is_distinct_utterance(previous: &str, next: &str) -> bool {
    let prev = previous.trim();
    let next_trimmed = next.trim();
    if prev.is_empty() || next_trimmed.is_empty() || prev == next_trimmed {
        return false;
    }
    !next_trimmed.starts_with(prev) && !prev.starts_with(next_trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::TranscriptEvent;

    fn base_event(direction: &str) -> TranscriptEvent {
        TranscriptEvent {
            direction: direction.to_string(),
            source_text: None,
            translated_text: None,
            interim: false,
            turn_complete: false,
            input_segment_finished: false,
            output_segment_finished: false,
            connection_gap: false,
            replace_live: false,
            live_source: None,
            live_translated: None,
        }
    }

    #[test]
    fn merge_streaming_text_appends_with_space() {
        assert_eq!(merge_streaming_text("Hello", "world"), "Hello world");
    }

    #[test]
    fn merge_streaming_text_replaces_with_longer_prefix() {
        assert_eq!(merge_streaming_text("Hello", "Hello world"), "Hello world");
    }

    #[test]
    fn ends_sentence_detects_period_and_trailing_quote() {
        assert!(ends_sentence(Some("Done.")));
        assert!(ends_sentence(Some("He said \"yes.\"")));
        assert!(!ends_sentence(Some("Still going")));
    }

    #[test]
    fn ends_sentence_detects_cjk_terminators() {
        assert!(ends_sentence(Some("完了。")));
        assert!(ends_sentence(Some("終わった！")));
        assert!(ends_sentence(Some("何？")));
        assert!(ends_sentence(Some("他说「好。」")));
        assert!(!ends_sentence(Some("还在说")));
    }

    #[test]
    fn is_distinct_utterance_requires_non_overlapping_text() {
        assert!(is_distinct_utterance(
            "First sentence.",
            "Second starts here"
        ));
        assert!(!is_distinct_utterance("Hello", "Hello world"));
    }

    #[test]
    fn commits_on_sentence_boundary() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1), ("inbound".to_string(), 1)]),
        );

        let mut event = base_event("outbound");
        event.source_text = Some("First.".to_string());
        engine.process(&event, true);

        event.source_text = Some("Second".to_string());
        let result = engine.process(&event, true);

        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.commits[0].source, "First.");
        assert_eq!(result.commits[0].sequence, 1);
        assert_eq!(result.commits[0].reason, CommitReason::Sentence);
        assert_eq!(result.live.source.as_deref(), Some("Second"));
    }

    #[test]
    fn commits_on_cjk_sentence_boundary() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1), ("inbound".to_string(), 1)]),
        );

        let mut event = base_event("outbound");
        event.source_text = Some("完了。".to_string());
        engine.process(&event, true);

        event.source_text = Some("次".to_string());
        let result = engine.process(&event, true);

        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.commits[0].source, "完了。");
        assert_eq!(result.commits[0].reason, CommitReason::Sentence);
        assert_eq!(result.live.source.as_deref(), Some("次"));
    }

    #[test]
    fn commits_on_turn_complete() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut event = base_event("outbound");
        event.source_text = Some("Done.".to_string());
        engine.process(&event, true);

        event.turn_complete = true;
        let result = engine.process(&event, true);

        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.commits[0].reason, CommitReason::Turn);
        assert!(result.live.source.is_none());
    }

    #[test]
    fn replace_live_revises_interim_without_appending() {
        let mut engine = SegmentEngine::new(0);
        let mut event = base_event("outbound");
        event.interim = true;
        event.replace_live = true;
        event.source_text = Some("How're you".to_string());
        engine.process(&event, true);

        event.source_text = Some("How are you".to_string());
        let result = engine.process(&event, true);

        assert_eq!(result.live.source.as_deref(), Some("How are you"));
    }

    #[test]
    fn connection_gap_commits_live_then_gap() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut event = base_event("outbound");
        event.source_text = Some("Partial".to_string());
        engine.process(&event, true);

        event.connection_gap = true;
        event.source_text = Some("[Connection interrupted]".to_string());
        let result = engine.process(&event, true);

        assert_eq!(result.commits.len(), 2);
        assert_eq!(result.commits[0].source, "Partial");
        assert!(result.commits[1].connection_gap);
    }

    #[test]
    fn flush_direction_commits_live_text() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut event = base_event("outbound");
        event.interim = true;
        event.replace_live = true;
        event.source_text = Some("Still talking".to_string());
        engine.process(&event, true);

        let commit = engine.flush_direction("outbound").expect("flush commit");
        assert_eq!(commit.source, "Still talking");
        assert_eq!(commit.reason, CommitReason::Flush);
        assert_eq!(commit.sequence, 1);
        assert!(engine.live_snapshot("outbound").source.is_none());
    }

    #[test]
    fn flush_seals_direction_against_late_turn_complete() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut event = base_event("outbound");
        event.interim = true;
        event.replace_live = true;
        event.source_text = Some("Trailing".to_string());
        engine.process(&event, true);
        let _ = engine.flush_direction("outbound").expect("flush");

        let mut late = base_event("outbound");
        late.turn_complete = true;
        late.source_text = Some("Trailing".to_string());
        let result = engine.process(&late, true);
        assert!(result.commits.is_empty());
        assert!(result.live.source.is_none());

        engine.open_direction("outbound");
        let mut again = base_event("outbound");
        again.interim = true;
        again.replace_live = true;
        again.source_text = Some("Fresh".to_string());
        let reopened = engine.process(&again, true);
        assert_eq!(reopened.live.source.as_deref(), Some("Fresh"));
    }

    #[test]
    fn flush_direction_noop_when_empty() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );
        assert!(engine.flush_direction("outbound").is_none());
        // Empty flush still seals.
        let mut late = base_event("outbound");
        late.turn_complete = true;
        late.source_text = Some("Should drop".to_string());
        assert!(engine.process(&late, true).commits.is_empty());
    }

    #[test]
    fn flush_all_both_directions() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1), ("inbound".to_string(), 1)]),
        );

        let mut out = base_event("outbound");
        out.source_text = Some("Hello from me".to_string());
        engine.process(&out, true);

        let mut inn = base_event("inbound");
        inn.source_text = Some("Hello from them".to_string());
        engine.process(&inn, true);

        let commits = engine.flush_all();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].direction, "outbound");
        assert_eq!(commits[0].reason, CommitReason::Flush);
        assert_eq!(commits[1].direction, "inbound");
        assert_eq!(commits[1].reason, CommitReason::Flush);
    }

    /// Regression: the same utterance delivered twice (overlapping sessions at
    /// start / reconnect replay) must not produce two identical segments.
    #[test]
    fn replayed_turn_complete_does_not_double_commit() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut interim = base_event("outbound");
        interim.interim = true;
        interim.replace_live = true;
        interim.source_text = Some("Hello there.".to_string());
        interim.translated_text = Some("Xin chào.".to_string());
        engine.process(&interim, true);

        let mut turn_end = base_event("outbound");
        turn_end.turn_complete = true;
        turn_end.replace_live = true;
        turn_end.source_text = Some("Hello there.".to_string());
        turn_end.translated_text = Some("Xin chào.".to_string());
        let first = engine.process(&turn_end, true);
        assert_eq!(first.commits.len(), 1);
        assert_eq!(first.commits[0].sequence, 1);

        // Second overlapping session replays the SAME utterance end-to-end.
        engine.process(&interim, true);
        let replayed = engine.process(&turn_end, true);
        assert!(
            replayed.commits.is_empty(),
            "replayed utterance must not commit a duplicate segment"
        );
    }

    /// A bare duplicate turn_complete (no interim in between) — e.g. the dying
    /// session's final frame arriving right after the live session's commit.
    #[test]
    fn bare_duplicate_turn_complete_does_not_double_commit() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut turn_end = base_event("outbound");
        turn_end.turn_complete = true;
        turn_end.replace_live = true;
        turn_end.source_text = Some("First words.".to_string());
        let first = engine.process(&turn_end, true);
        assert_eq!(first.commits.len(), 1);

        let second = engine.process(&turn_end, true);
        assert!(
            second.commits.is_empty(),
            "bare duplicate turn_complete must not commit again"
        );
    }

    /// Legit consecutive repeats far apart in time still commit — the guard
    /// must only catch immediate re-delivery, not real speech.
    #[test]
    fn identical_repeat_after_window_still_commits() {
        let mut engine = SegmentEngine::new(0);
        engine.reset_for_meeting(
            Some("m1".to_string()),
            0,
            HashMap::from([("outbound".to_string(), 1)]),
        );

        let mut turn_end = base_event("outbound");
        turn_end.turn_complete = true;
        turn_end.replace_live = true;
        turn_end.source_text = Some("Yeah.".to_string());
        assert_eq!(engine.process(&turn_end, true).commits.len(), 1);

        // Simulate time passing beyond the duplicate-delivery window.
        engine.debug_age_last_commit_for_test();

        let repeat = engine.process(&turn_end, true);
        assert_eq!(
            repeat.commits.len(),
            1,
            "a genuine repeat after the window must still commit"
        );
    }
}
