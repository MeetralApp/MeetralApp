//! ElevenLabs TTS debug — delivery queue and WebSocket wire.

use super::enabled;

/// Text queued toward the ElevenLabs worker (`AppendDelta`).
pub fn log_elevenlabs_queue_text(reason: &str, text: &str) {
    if !enabled() {
        return;
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    tracing::info!(
        backend = "elevenlabs",
        reason,
        chars = trimmed.chars().count(),
        text = %trimmed,
        "elevenlabs-queue-text"
    );
}

/// Delivery asked the worker to send ElevenLabs `{ flush: true }` (session stays open).
pub fn log_elevenlabs_queue_flush(reason: &str, text: &str) {
    if !enabled() {
        return;
    }
    let trimmed = text.trim();
    tracing::info!(
        backend = "elevenlabs",
        reason,
        flush = true,
        chars = trimmed.chars().count(),
        text = %trimmed,
        "elevenlabs-queue-flush"
    );
}

pub fn log_elevenlabs_ws_init(chunk_schedule: Option<[u32; 4]>, auto_mode: bool) {
    if !enabled() {
        return;
    }
    match chunk_schedule {
        Some(schedule) => {
            tracing::info!(
                backend = "elevenlabs",
                schedule = ?schedule,
                auto_mode,
                "elevenlabs-ws-init with chunk_length_schedule"
            );
        }
        None => {
            tracing::info!(
                backend = "elevenlabs",
                auto_mode,
                "elevenlabs-ws-init without generation_config (natural mode)"
            );
        }
    }
}

/// Wire: `{ text, try_trigger_generation }`.
pub fn log_elevenlabs_ws_text(text: &str, try_trigger_generation: bool) {
    if !enabled() {
        return;
    }
    tracing::info!(
        backend = "elevenlabs",
        chars = text.chars().count(),
        try_trigger_generation,
        text = %text.trim(),
        "elevenlabs-ws-text"
    );
}

/// Wire: `{ text: " ", flush: true }` — force generate; connection remains open.
pub fn log_elevenlabs_ws_flush() {
    if !enabled() {
        return;
    }
    tracing::info!(backend = "elevenlabs", flush = true, "elevenlabs-ws-flush");
}

pub fn log_elevenlabs_ws_is_final() {
    if !enabled() {
        return;
    }
    tracing::info!(
        backend = "elevenlabs",
        is_final = true,
        "elevenlabs-ws-is-final"
    );
}
