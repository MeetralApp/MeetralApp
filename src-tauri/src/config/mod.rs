pub mod app_config;
pub mod app_config_setup;
pub mod app_config_validate;
pub mod app_config_voice;
pub mod custom_llm;
pub mod device;
pub mod elevenlabs_public;
pub mod elevenlabs_settings;
pub mod meeting_context;
pub mod modes;
pub mod overlay_settings;
pub mod soniox_settings;
pub mod view;

#[cfg(test)]
mod tests;

pub use app_config::AppConfig;
pub use custom_llm::{
    normalize_custom_llm_profile, CustomLlmProfile, CustomLlmProfileView, LlmSelection,
};
pub use device::DeviceRef;
pub use elevenlabs_settings::ElevenLabsSettings;
pub use meeting_context::{
    MeetingContextPair, MeetingContextPayload, MeetingContextTranslationTerm,
};
pub use modes::{
    AudioPathMode, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode, SessionMode,
    ThemePreference, TranscriptLayout, VadSensitivity,
};
pub use overlay_settings::{
    default_overlay_height, default_overlay_width, OverlayPosition, OverlaySettings,
    OVERLAY_MAX_HEIGHT, OVERLAY_MAX_WIDTH, OVERLAY_MIN_HEIGHT, OVERLAY_MIN_WIDTH,
};
pub use soniox_settings::SonioxSettings;
pub use view::ConfigView;

pub const INPUT_SAMPLE_RATE: u32 = 48000;
pub const OUTPUT_SAMPLE_RATE: u32 = 24000;
pub const FRAME_MS: u32 = 100;
