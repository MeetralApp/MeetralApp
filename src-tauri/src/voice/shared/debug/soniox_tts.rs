//! Soniox TTS WebSocket debug — only emitted by the Soniox TTS worker.

use super::enabled;

/// Soniox TTS WebSocket lifecycle (`worker_ready_lazy`, `first_audio`, …).
pub fn log_soniox_tts(event: &str) {
    if !enabled() {
        return;
    }
    tracing::info!(backend = "soniox", event, "soniox-tts");
}

/// Wire: `{ text, text_end: false, stream_id }`.
pub fn log_soniox_tts_ws_text(chars: usize) {
    if !enabled() {
        return;
    }
    tracing::info!(
        backend = "soniox",
        text_end = false,
        chars,
        "soniox-tts-ws-text"
    );
}

/// Wire: `{ text: "", text_end: true, stream_id }` — stream termination handshake.
pub fn log_soniox_tts_ws_text_end() {
    if !enabled() {
        return;
    }
    tracing::info!(
        backend = "soniox",
        text_end = true,
        "soniox-tts-ws-text-end"
    );
}
