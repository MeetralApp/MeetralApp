use std::sync::atomic::{AtomicU64, AtomicU8};

use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::providers::elevenlabs::delivery::core::last_committable_sentence_end;
use crate::providers::elevenlabs::delivery::core::{
    is_junk_tts_fragment, merge_streaming_text, normalize_interim_translated,
    streaming_text_ends_sentence,
};
use crate::providers::elevenlabs::delivery::state::RelayState;
use crate::providers::elevenlabs::delivery::tts::append_delta;
use crate::providers::elevenlabs::delivery::tts::flush_speed_sentence_boundary;
use crate::providers::shared::live::TranscriptEvent;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;

use super::constants::{
    FAST_LANE_INTERJECTION_MAX_CHARS, FAST_LANE_MAX_CHARS, SPEED_SENTENCE_MIN_CHARS,
};

pub(crate) fn segment_fast_lane(event: &TranscriptEvent) -> bool {
    if !(event.output_segment_finished && event.input_segment_finished) {
        return false;
    }
    let Some(text) = event.translated_text.as_deref() else {
        return false;
    };
    let chars = text.trim().chars().count();
    if !(1..=FAST_LANE_MAX_CHARS).contains(&chars) {
        return false;
    }
    streaming_text_ends_sentence(text) || chars <= FAST_LANE_INTERJECTION_MAX_CHARS
}

pub(crate) fn ensure_segment_text_on_el(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    state: &mut RelayState,
) {
    if state.speed.has_unflushed_text {
        return;
    }
    let pending = state.tracker.last_translated().trim().to_string();
    if pending.is_empty() {
        return;
    }
    append_delta(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        format!("{pending} "),
        state,
    );
}

pub(crate) fn ingest_translated_interim(
    translated: &str,
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    turn_latency: &TurnLatencySlot,
    state: &mut RelayState,
    revision_debounce_at: &mut Option<Instant>,
    sentence_flush_at: &mut Option<Instant>,
) {
    let normalized = normalize_interim_translated(translated);
    state.stamp_first_translated(turn_latency);

    let merged = merge_streaming_text(state.tracker.last_translated(), &normalized);

    if state.speed.revision_pending() {
        let _ = state.tracker.append_translated(&merged);
        state.speed.arm_revision_debounce(merged);
        *revision_debounce_at = state.speed.revision_deadline();
        return;
    }

    let Some((delta, needs_flush)) = state.tracker.append_translated(&merged) else {
        return;
    };

    if needs_flush {
        state
            .speed
            .arm_revision_debounce(state.tracker.last_translated().to_string());
        *revision_debounce_at = state.speed.revision_deadline();
        return;
    }

    if !is_junk_tts_fragment(&delta) {
        append_delta(
            tts_cmd_tx,
            voice_engine,
            relay_chars_while_provider,
            delta,
            state,
        );
        *sentence_flush_at = state.speed.arm_sentence_flush(&state.tracker);
    }
}

pub(crate) fn apply_speed_sentence_flush(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    state: &mut RelayState,
) {
    let boundary_end = state.speed.take_pending_sentence_end().or_else(|| {
        let text = state.tracker.last_translated();
        last_committable_sentence_end(text).filter(|end| {
            *end > state.speed.last_sentence_flush_byte
                && text[state.speed.last_sentence_flush_byte..*end]
                    .chars()
                    .count()
                    >= SPEED_SENTENCE_MIN_CHARS
        })
    });
    let Some(end) = boundary_end else {
        return;
    };
    flush_speed_sentence_boundary(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        state,
        end,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::elevenlabs::delivery::state::RelayState;
    use crate::providers::shared::live::TranscriptEvent;
    use crate::voice::config::VOICE_ENGINE_PROVIDER;
    use crate::voice::shared::latency::TurnLatencySlot;
    use crate::voice::shared::tts_command::TtsTextCommand;

    fn event_with(
        translated: &str,
        turn_complete: bool,
        input_finished: bool,
        output_finished: bool,
    ) -> TranscriptEvent {
        TranscriptEvent {
            direction: "outbound".to_string(),
            source_text: None,
            translated_text: Some(translated.to_string()),
            interim: !turn_complete,
            turn_complete,
            input_segment_finished: input_finished,
            output_segment_finished: output_finished,
            connection_gap: false,
            replace_live: false,
            live_source: None,
            live_translated: None,
        }
    }

    #[test]
    fn turn_complete_event_detected() {
        assert!(event_with("Yes", true, true, true).turn_complete);
    }

    #[test]
    fn segment_fast_lane_detects_short_interjection() {
        assert!(segment_fast_lane(&event_with("Yes", false, true, true)));
    }

    #[test]
    fn segment_fast_lane_skips_long_partial() {
        let long = "This is a longer sentence that should wait for turn complete.";
        assert!(!segment_fast_lane(&event_with(long, false, true, true)));
    }

    #[test]
    fn segment_fast_lane_requires_sentence_end_or_short_interjection() {
        assert!(segment_fast_lane(&event_with("Yes", false, true, true)));
        assert!(segment_fast_lane(&event_with("OK.", false, true, true)));
        let long = "This is a longer sentence that should wait for turn complete.";
        assert!(!segment_fast_lane(&event_with(long, false, true, true)));
        let long_no_period = "This is a longer sentence without ending punctuation";
        assert!(!segment_fast_lane(&event_with(
            long_no_period,
            false,
            true,
            true
        )));
    }

    #[test]
    fn speed_sentence_flush_arms_after_complete_sentence() {
        let (tx, _rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let turn = TurnLatencySlot::new_shared();
        let mut state = RelayState::new();
        let mut revision_at = None;
        let mut sentence_at = None;

        ingest_translated_interim(
            "Hello world. Next",
            &tx,
            &engine,
            &chars,
            &turn,
            &mut state,
            &mut revision_at,
            &mut sentence_at,
        );
        assert!(sentence_at.is_some());
        assert!(state.speed.has_unflushed_text);
    }

    #[test]
    fn speed_sentence_boundary_flush_keeps_tracker() {
        let (tx, mut rx) = mpsc::channel::<TtsTextCommand>(16);
        let tx = std::sync::Mutex::new(tx);
        let engine = AtomicU8::new(VOICE_ENGINE_PROVIDER);
        let chars = AtomicU64::new(0);
        let mut state = RelayState::new();
        state.tracker.set_buffer("Hello world. Next");
        state.speed.has_unflushed_text = true;
        state.speed.el_sent_anchor = "Hello world. Next".to_string();
        assert!(state.speed.arm_sentence_flush(&state.tracker).is_some());
        apply_speed_sentence_flush(&tx, &engine, &chars, &mut state);

        assert!(rx.try_recv().is_ok());
        assert!(!state.speed.has_unflushed_text);
        assert_eq!(state.tracker.last_translated(), "Hello world. Next");
        assert!(state.speed.last_sentence_flush_byte > 0);
    }
}
