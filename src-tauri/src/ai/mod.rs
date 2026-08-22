//! Third-party AI provider configuration and live translation.
//!
//! Prefer `crate::capabilities` for caps/catalog/routing and `crate::providers`
//! for vendor config. This module owns `AiProvider` and summary; live-bridge
//! types are re-exported from `providers::shared::live` for compatibility.

pub mod language_flags;
pub mod llm;
pub mod provider;
pub mod summary;
pub mod types;

pub use language_flags::{
    country_code_for_language, primary_language_code, resolve_country_code,
    LANGUAGE_FLAG_COUNTRY_CODES,
};

pub use crate::capabilities::{
    bridge_emits_playback_audio, bridge_play_audio_enabled,
    catalog_supported_languages_for_provider, clamp_to_live_catalog,
    default_live_model_for_provider, default_summary_model_for_provider, get_provider_catalog,
    is_supported_language_for_provider, languages_for_live_model, live_caps,
    migrate_config_for_provider, normalize_live_model_for_provider,
    normalize_summary_model_for_provider, outbound_fanout_kind, outbound_playback_source,
    scaffolds_provider_tts, seed_live_models, tts_text_pipeline_active,
    uses_provider_tts_for_inbound, uses_provider_tts_for_outbound, uses_separate_tts, FanoutKind,
    PlaybackSource, ProviderCapabilities, ProviderCatalog,
};
pub use crate::providers::gemini::config::{
    ai_catalog, catalog_gemini_languages, catalog_gemini_live_models,
    catalog_gemini_summary_models, default_gemini_live_model, default_gemini_summary_model,
    is_gemini_supported_language, live_ws_url, normalize_gemini_live_model,
    normalize_gemini_summary_model, rest_generate_content_url, DEFAULT_LIVE_MODEL,
    DEFAULT_SUMMARY_MODEL, GEMINI_API_VERSION, GEMINI_REST_BASE, GEMINI_REST_TIMEOUT_SECS,
};
pub use crate::providers::openai::config::{
    catalog_openai_languages, catalog_openai_live_models, catalog_openai_summary_models,
    chat_completions_url, default_openai_live_model, default_openai_summary_model,
    normalize_openai_live_model, normalize_openai_summary_model, translations_ws_url,
    ALLOWED_OPENAI_LIVE_MODELS, ALLOWED_OPENAI_SUMMARY_MODELS, DEFAULT_OPENAI_LIVE_MODEL,
    DEFAULT_OPENAI_SUMMARY_MODEL, OPENAI_OUTPUT_LANGUAGES, OPENAI_REST_BASE,
    OPENAI_TRANSLATIONS_WS,
};
pub use crate::providers::shared::live::{
    decode_ws_message, BridgeFatalSender, BridgeStatusEvent, BridgeStatusSender, LiveBridge,
    LiveBridgeHandle, LiveSetupOptions, ReconnectPolicy, TranscriptEvent, TranscriptSender,
    MAX_RECONNECT_ATTEMPTS,
};
pub use llm::{ChatFuture, ChatLlmProvider, ChatRequest, LlmError, LlmErrorKind};
pub use provider::AiProvider;
pub use summary::SummaryLlmClient;
pub use types::{AiCatalog, AiCatalogDefaults, AiModelInfo, LanguageInfo, LiveModelOption};
