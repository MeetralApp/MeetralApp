//! Capability traits and catalog surface for provider-agnostic runtime.

pub mod catalog;
pub mod tts;

pub use catalog::{
    catalog_supported_languages_for_provider, clamp_to_live_catalog,
    default_live_model_for_provider, default_summary_model_for_provider, get_provider_catalog,
    is_supported_language_for_provider, languages_for_live_model, llm_capabilities_for,
    migrate_config_for_provider, normalize_live_model_for_provider,
    normalize_summary_model_for_provider, seed_live_models, LlmAuthMode, LlmBaseUrl,
    LlmCapabilities, ModelSource, ProviderCapabilities, ProviderCatalog, LLM_CAPS_COMPATIBLE,
    LLM_CAPS_GEMINI, LLM_CAPS_OPENAI,
};
pub use tts::{
    bridge_emits_playback_audio, bridge_play_audio_enabled, inbound_fanout_kind,
    inbound_playback_source, live_caps, needs_elevenlabs_for_inbound,
    needs_elevenlabs_for_outbound, outbound_fanout_kind, outbound_playback_source,
    scaffolds_inbound_text_tts, scaffolds_provider_tts, tts_text_pipeline_active,
    uses_provider_tts_for_inbound, uses_provider_tts_for_outbound, uses_separate_tts, FanoutKind,
    PlaybackSource,
};

#[cfg(test)]
mod tests;
