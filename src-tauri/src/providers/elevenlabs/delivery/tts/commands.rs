use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use tokio::sync::mpsc;
use tracing::info;

use crate::voice::config::VOICE_ENGINE_PROVIDER;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCloneLatencyEvent;

use super::types::FlushReason;
use crate::providers::elevenlabs::delivery::state::RelayState;

pub(crate) fn send_tts_cmd(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    el_text_chars_while_provider: &AtomicU64,
    cmd: TtsTextCommand,
) {
    let engine = voice_engine.load(Ordering::SeqCst);
    if engine == VOICE_ENGINE_PROVIDER {
        if let TtsTextCommand::AppendDelta { ref text, .. } = cmd {
            let chars = text.chars().count() as u64;
            if chars > 0 {
                el_text_chars_while_provider.fetch_add(chars, Ordering::Relaxed);
            }
        }
    }
    if let Ok(tx) = tts_cmd_tx.lock() {
        crate::runtime::control_channel::try_send_control(&tx, cmd, "tts-cmd");
    }
}

pub(crate) fn emit_turn_latency(
    latency_tx: &Option<mpsc::Sender<VoiceCloneLatencyEvent>>,
    turn_latency: &TurnLatencySlot,
    flush_reason: FlushReason,
) {
    let flush_ms = crate::audio::monotonic_ms();
    let Some(event) = turn_latency.build_flush_event(flush_ms, flush_reason.as_str()) else {
        return;
    };
    info!(
        flush_reason = flush_reason.as_str(),
        translate_ms = event.translate_ms,
        tts_ms = event.tts_ms,
        total_ms = event.total_ms,
        "voice-clone-latency"
    );
    if let Some(tx) = latency_tx {
        crate::runtime::control_channel::try_send_control(tx, event, "voice-latency");
    }
    turn_latency.reset();
}

pub(crate) fn append_delta(
    tts_cmd_tx: &std::sync::Mutex<mpsc::Sender<TtsTextCommand>>,
    voice_engine: &AtomicU8,
    relay_chars_while_provider: &AtomicU64,
    text: String,
    state: &mut RelayState,
) {
    let chars = text.chars().count();
    send_tts_cmd(
        tts_cmd_tx,
        voice_engine,
        relay_chars_while_provider,
        TtsTextCommand::AppendDelta {
            text,
            trigger_generation: false,
        },
    );
    state.speed.has_unflushed_text = true;
    state.speed.chars_since_last_flush += chars;
    state.speed.el_sent_anchor = state.tracker.last_translated().to_string();
}
