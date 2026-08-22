//! Opt-in Soniox STT trace logs (`DEBUG_MODE=1`).

use crate::debug_mode;

/// Soniox STT endpoint (`<end>`) — one speaking turn finalized.
pub fn log_soniox_endpoint(direction: &str, source_chars: usize, translated_chars: usize) {
    if !debug_mode::enabled() {
        return;
    }
    tracing::info!(
        backend = "soniox",
        direction,
        source_chars,
        translated_chars,
        "soniox-stt-endpoint-end"
    );
}

pub fn log_soniox_stt(direction: &str, event: &str) {
    if !debug_mode::enabled() {
        return;
    }
    tracing::info!(backend = "soniox", direction, event, "soniox-stt");
}
