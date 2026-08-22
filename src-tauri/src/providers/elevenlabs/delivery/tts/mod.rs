//! TTS command sink — append, flush, and latency emission.

mod commands;
mod flush;
mod types;

pub(crate) use commands::{append_delta, emit_turn_latency, send_tts_cmd};
pub(crate) use flush::{
    commit_sentence_chunk, flush_pending_before_reset, flush_speed_sentence_boundary, flush_tts,
};
pub(crate) use types::{FlushReason, ReconcileAction};
