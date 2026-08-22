//! Turn-scoped prefix-delta delivery for Soniox AI → TTS.
//!
//! Streams interim suffixes as soon as they arrive and flushes once per turn.
//! Used for both ElevenLabs Clone and Soniox Provider TTS.

use std::sync::Mutex;

use tokio::sync::mpsc;
use tracing::info;

use crate::voice::shared::debug;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCloneLatencyEvent;

pub(crate) struct StreamState {
    last_translated: String,
    has_unflushed: bool,
    phrase_started: bool,
}

impl StreamState {
    pub(crate) fn new() -> Self {
        Self {
            last_translated: String::new(),
            has_unflushed: false,
            phrase_started: false,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.last_translated.clear();
        self.has_unflushed = false;
        self.phrase_started = false;
    }

    pub(crate) fn has_unflushed(&self) -> bool {
        self.has_unflushed
    }

    fn stamp_phrase_start(&mut self, turn_latency: &TurnLatencySlot) {
        if self.phrase_started {
            return;
        }
        let now = crate::audio::monotonic_ms();
        turn_latency.begin_turn(now, now);
        self.phrase_started = true;
    }
}

fn send_cmd(tts_cmd_tx: &Mutex<mpsc::Sender<TtsTextCommand>>, cmd: TtsTextCommand) {
    if let Ok(tx) = tts_cmd_tx.lock() {
        crate::runtime::control_channel::try_send_control(&tx, cmd, "tts-cmd");
    }
}

pub(crate) fn ingest_stream_delta(
    translated: &str,
    tts_cmd_tx: &Mutex<mpsc::Sender<TtsTextCommand>>,
    turn_latency: &TurnLatencySlot,
    state: &mut StreamState,
) {
    let trimmed = translated.trim();
    if trimmed.is_empty() {
        return;
    }
    if trimmed == state.last_translated {
        return;
    }

    state.stamp_phrase_start(turn_latency);

    if trimmed.starts_with(&state.last_translated) {
        let delta = trimmed[state.last_translated.len()..].to_string();
        state.last_translated = trimmed.to_string();
        if delta.is_empty() {
            return;
        }
        debug::log_soniox_delivery_delta("interim", &delta);
        send_cmd(
            tts_cmd_tx,
            TtsTextCommand::AppendDelta {
                text: delta,
                trigger_generation: false,
            },
        );
        state.has_unflushed = true;
        return;
    }

    // Non-prefix revision: cancel current stream, then speak the new full text.
    debug::log_soniox_delivery("revision_reset");
    send_cmd(tts_cmd_tx, TtsTextCommand::Reset);
    state.reset();
    state.stamp_phrase_start(turn_latency);
    debug::log_soniox_delivery_delta("revision", trimmed);
    send_cmd(
        tts_cmd_tx,
        TtsTextCommand::AppendDelta {
            text: trimmed.to_string(),
            trigger_generation: false,
        },
    );
    state.last_translated = trimmed.to_string();
    state.has_unflushed = true;
}

pub(crate) fn flush_stream_turn(
    tts_cmd_tx: &Mutex<mpsc::Sender<TtsTextCommand>>,
    state: &mut StreamState,
    latency_tx: &Option<mpsc::Sender<VoiceCloneLatencyEvent>>,
    turn_latency: &TurnLatencySlot,
) {
    if !state.has_unflushed {
        state.reset();
        return;
    }
    debug::log_soniox_delivery_flush(
        "turn_complete",
        state.last_translated.chars().count(),
        &state.last_translated,
    );
    send_cmd(tts_cmd_tx, TtsTextCommand::Flush);
    state.has_unflushed = false;

    let flush_ms = crate::audio::monotonic_ms();
    if let Some(event) = turn_latency.build_flush_event(flush_ms, "turn_complete") {
        if crate::debug_mode::enabled() {
            info!(
                flush_reason = "turn_complete",
                translate_ms = event.translate_ms,
                tts_ms = event.tts_ms,
                total_ms = event.total_ms,
                "soniox-delivery-latency"
            );
        }
        if let Some(tx) = latency_tx {
            crate::runtime::control_channel::try_send_control(tx, event, "voice-latency");
        }
    }
    turn_latency.reset();
    state.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_delta_only_appends_suffix() {
        let (tx, mut rx) = mpsc::channel(16);
        let cmd_tx = Mutex::new(tx);
        let mut state = StreamState::new();
        let latency = TurnLatencySlot::new_shared();

        ingest_stream_delta("Hello", &cmd_tx, &latency, &mut state);
        ingest_stream_delta("Hello world", &cmd_tx, &latency, &mut state);

        match rx.try_recv().unwrap() {
            TtsTextCommand::AppendDelta { text, .. } => assert_eq!(text, "Hello"),
            other => panic!("unexpected {other:?}"),
        }
        match rx.try_recv().unwrap() {
            TtsTextCommand::AppendDelta { text, .. } => assert_eq!(text, " world"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(state.has_unflushed());
    }

    #[test]
    fn non_prefix_sends_reset_then_full_text() {
        let (tx, mut rx) = mpsc::channel(16);
        let cmd_tx = Mutex::new(tx);
        let mut state = StreamState::new();
        let latency = TurnLatencySlot::new_shared();

        ingest_stream_delta("Hello", &cmd_tx, &latency, &mut state);
        let _ = rx.try_recv();
        ingest_stream_delta("Hi there", &cmd_tx, &latency, &mut state);

        assert!(matches!(rx.try_recv().unwrap(), TtsTextCommand::Reset));
        match rx.try_recv().unwrap() {
            TtsTextCommand::AppendDelta { text, .. } => assert_eq!(text, "Hi there"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn turn_complete_flushes_once() {
        let (tx, mut rx) = mpsc::channel(16);
        let cmd_tx = Mutex::new(tx);
        let mut state = StreamState::new();
        let latency = TurnLatencySlot::new_shared();

        ingest_stream_delta("Hello", &cmd_tx, &latency, &mut state);
        let _ = rx.try_recv();
        flush_stream_turn(&cmd_tx, &mut state, &None, &latency);

        assert!(matches!(rx.try_recv().unwrap(), TtsTextCommand::Flush));
        assert!(!state.has_unflushed());
        assert!(state.last_translated.is_empty());
    }
}
