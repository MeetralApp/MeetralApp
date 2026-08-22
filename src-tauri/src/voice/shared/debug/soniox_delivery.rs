//! Soniox STT transcript → TTS command queue (not Soniox TTS wire).
//!
//! Used whenever AI provider is Soniox. The attached TTS worker may be
//! ElevenLabs (Clone) or Soniox TTS (Provider). Wire proof is
//! `soniox-tts-ws-*` / `elevenlabs-ws-*`.
//!
//! Delivery streams prefix deltas and flushes once per turn (low TTFB).

use super::{enabled, preview};

/// Interim suffix → `TtsTextCommand::AppendDelta`.
pub fn log_soniox_delivery_delta(source: &str, delta: &str) {
    if !enabled() {
        return;
    }
    let trimmed = delta.trim();
    if trimmed.is_empty() {
        return;
    }
    tracing::info!(
        stage = "delivery",
        ai_provider = "soniox",
        source,
        chars = trimmed.chars().count(),
        text = %trimmed,
        "soniox-delivery-delta"
    );
}

/// Turn complete → `TtsTextCommand::Flush` to the active TTS worker.
///
/// ElevenLabs maps Flush → `{flush:true}`; Soniox TTS maps Flush → `text_end:true`.
pub fn log_soniox_delivery_flush(reason: &str, turn_chars: usize, turn_text: &str) {
    if !enabled() {
        return;
    }
    tracing::info!(
        stage = "delivery",
        ai_provider = "soniox",
        reason,
        turn_chars,
        preview = %preview(turn_text, 48),
        "soniox-delivery-flush"
    );
}

/// Delivery-layer lifecycle (`connection_gap_reset`, …) — not Soniox TTS wire.
pub fn log_soniox_delivery(event: &str) {
    if !enabled() {
        return;
    }
    tracing::info!(
        stage = "delivery",
        ai_provider = "soniox",
        event,
        "soniox-delivery"
    );
}
