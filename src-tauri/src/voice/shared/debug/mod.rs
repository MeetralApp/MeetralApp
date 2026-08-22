//! Opt-in voice / STT-adjacent trace logs for local debugging.
//!
//! Gated by [`crate::debug_mode`] (`DEBUG_MODE=1` in `.env`).
//! Release builds never emit these lines.
//!
//! Split by stage / backend so log helpers are not mistaken for a shared wire API:
//! - [`elevenlabs`] — ElevenLabs queue + WebSocket
//! - [`soniox_delivery`] — Soniox STT transcript → TTS command queue (any TTS worker)
//! - [`soniox_tts`] — Soniox TTS WebSocket only

mod elevenlabs;
mod soniox_delivery;
mod soniox_tts;

pub use elevenlabs::*;
pub use soniox_delivery::*;
pub use soniox_tts::*;

use crate::debug_mode;

/// Shared debug gate for voice trace logs.
pub fn enabled() -> bool {
    debug_mode::enabled()
}

pub(crate) fn preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_string();
    }
    let tail: String = trimmed
        .chars()
        .skip(count.saturating_sub(max_chars))
        .collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_gate_is_callable() {
        let _ = enabled();
    }

    #[test]
    fn preview_truncates_long_text() {
        let p = preview("abcdefghijklmnopqrstuvwxyz", 8);
        assert_eq!(p, "…stuvwxyz");
    }
}
