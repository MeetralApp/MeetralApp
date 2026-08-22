use std::sync::atomic::{AtomicU64, AtomicU8};

use tokio::sync::mpsc;

use crate::voice::shared::debug;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCloneLatencyEvent;

use super::commands::{append_delta, emit_turn_latency, send_tts_cmd};
use super::types::FlushReason;
use crate::providers::elevenlabs::delivery::state::RelayState;

pub(crate) fn flush_tts(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    latency_tx: &Option<mpsc::Sender<VoiceCloneLatencyEvent>>,
    turn_latency: &TurnLatencySlot,
    reason: FlushReason,
    hard: bool,
    state: &mut RelayState,
) {
    state.record_flush();
    send_tts_cmd(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        TtsTextCommand::Flush,
    );
    if hard {
        if reason == FlushReason::TurnComplete {
            state.log_turn_metrics();
            state.reset_turn_metrics();
        }
        emit_turn_latency(latency_tx, turn_latency, reason);
        let anchor = state.tracker.last_translated().to_string();
        state.reset_phrase(turn_latency);
        if matches!(
            reason,
            FlushReason::SegmentFastLane | FlushReason::OpenAiIdleSafety
        ) {
            state.reanchor(&anchor);
        }
    }
    state.speed.chars_since_last_flush = 0;
    state.speed.has_unflushed_text = false;
}

/// Mid-turn sentence flush for Speed mode — keeps tracker state for continued interim.
pub(crate) fn flush_speed_sentence_boundary(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    state: &mut RelayState,
    boundary_end: usize,
) {
    let text = state.tracker.last_translated();
    let snippet = text[..boundary_end.min(text.len())].trim();
    debug::log_elevenlabs_queue_flush(FlushReason::SentenceBoundary.as_str(), snippet);
    state.record_flush();
    send_tts_cmd(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        TtsTextCommand::Flush,
    );
    state.speed.has_unflushed_text = false;
    state.speed.chars_since_last_flush = 0;
    state.speed.last_sentence_flush_byte = boundary_end;
    state.speed.clear_pending_sentence_end();
}

/// Flush any buffered audio before closing the ElevenLabs websocket.
pub(crate) fn flush_pending_before_reset(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    state: &mut RelayState,
) {
    if !state.speed.has_unflushed_text {
        return;
    }
    send_tts_cmd(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        TtsTextCommand::Flush,
    );
    state.speed.has_unflushed_text = false;
    state.speed.chars_since_last_flush = 0;
}

pub(crate) fn commit_sentence_chunk(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    chunk: &str,
    flush_reason: &str,
    state: &mut RelayState,
) {
    debug::log_elevenlabs_queue_text(flush_reason, chunk);
    append_delta(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        chunk.to_string(),
        state,
    );
    send_tts_cmd(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        TtsTextCommand::Flush,
    );
    debug::log_elevenlabs_queue_flush(flush_reason, chunk);
    state.record_flush();
    state.speed.has_unflushed_text = false;
    state.speed.chars_since_last_flush = 0;
}

#[cfg(test)]
mod tests {
    use crate::voice::shared::latency::TurnLatencySlot;

    #[test]
    fn turn_latency_builds_event_on_flush() {
        let slot = TurnLatencySlot::new_shared();
        slot.begin_turn(1000, 1200);
        slot.seed_first_audio_ms(1300);
        let event = slot.build_flush_event(1500, "turn").expect("event");
        assert_eq!(event.flush_reason, "turn");
        assert_eq!(event.flush_ms, 1500);
        assert_eq!(event.translate_ms, 200);
        assert_eq!(event.tts_ms, 100);
        assert_eq!(event.total_ms, 300);
    }
}
