//! Vertical provider slices — live AI + TTS backends.
//!
//! `match AiProvider` for vendor-specific protocol/config lives here.
//! Session wiring must go through `runtime::factories` instead.

pub mod compatible;
pub mod elevenlabs;
pub mod fishaudio;
pub mod gemini;
pub mod openai;
pub mod shared;
pub mod soniox;
pub mod xai;

use crate::ai::provider::AiProvider;

pub fn live_upload_sample_rate(provider: AiProvider) -> u32 {
    match provider {
        AiProvider::Gemini => gemini::config::UPLOAD_SAMPLE_RATE,
        AiProvider::OpenAi => openai::config::UPLOAD_SAMPLE_RATE,
        AiProvider::Soniox => soniox::config::UPLOAD_SAMPLE_RATE,
    }
}
