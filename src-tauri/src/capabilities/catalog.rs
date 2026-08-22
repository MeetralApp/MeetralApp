use serde::{Deserialize, Serialize};

use crate::ai::provider::AiProvider;
use crate::ai::types::{AiCatalogDefaults, AiModelInfo, LanguageInfo, LiveModelOption};
use crate::providers::gemini::config::{
    catalog_gemini_languages, catalog_gemini_live_models, catalog_gemini_summary_models,
    default_gemini_live_model, default_gemini_summary_model, is_gemini_supported_language,
    normalize_gemini_live_model, normalize_gemini_summary_model, UPLOAD_SAMPLE_RATE,
};
use crate::providers::openai::config::{
    catalog_openai_languages, catalog_openai_live_models, catalog_openai_notes_stt_model,
    catalog_openai_summary_models, default_openai_live_model, default_openai_summary_model,
    is_openai_supported_language, normalize_openai_live_model, normalize_openai_summary_model,
    UPLOAD_SAMPLE_RATE as OPENAI_UPLOAD_SAMPLE_RATE,
};
use crate::providers::soniox::config::{
    catalog_soniox_languages, catalog_soniox_live_models, catalog_soniox_summary_models,
    default_soniox_live_model, default_soniox_summary_model, is_soniox_supported_language,
    normalize_soniox_live_model, normalize_soniox_summary_model,
    UPLOAD_SAMPLE_RATE as SONIOX_UPLOAD_SAMPLE_RATE,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub supports_vad_config: bool,
    pub supports_echo_target_language: bool,
    pub live_upload_sample_rate: u32,
    #[serde(default)]
    pub supports_native_summary: bool,
    #[serde(default)]
    pub uses_separate_tts: bool,
    /// Live bridge WebSocket emits playback PCM (STS). False when TTS is a separate API.
    pub bridge_emits_playback_audio: bool,
    /// Session Mode Notes (STT-only, no MT). False when live API always requires translation.
    #[serde(default)]
    pub supports_notes_stt_only: bool,
}

/// How a chat LLM authenticates — drives gate logic (no special-casing providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlmAuthMode {
    /// API key mandatory (cloud vendors).
    Required,
    /// Key accepted but not required (OpenAI-compatible local servers).
    Optional,
    /// No credential at all.
    None,
}

/// Whether the endpoint is vendor-fixed or user-configurable (custom profiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlmBaseUrl {
    Fixed,
    Configurable,
}

/// Where chat model ids come from — allowlist clamp or freeform text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelSource {
    Allowlist,
    Freeform,
}

/// Capability descriptor for a chat LLM backend. Callers degrade by
/// capability instead of special-casing providers: `json_mode = false` →
/// prompt-only JSON + tolerant parse; `streaming = false` → non-stream path;
/// `auth = None` → gate satisfied once a base URL is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmCapabilities {
    pub streaming: bool,
    pub json_mode: bool,
    pub auth: LlmAuthMode,
    pub base_url: LlmBaseUrl,
    pub model_source: ModelSource,
    /// Vendor-published context window when known (informational only).
    pub context_window_tokens: Option<u32>,
    /// Reserved (backlog) — no provider reports reasoning yet.
    pub reasoning: bool,
    /// Reserved (backlog) — no provider reports tool calling yet.
    pub tool_calling: bool,
}

pub const LLM_CAPS_GEMINI: LlmCapabilities = LlmCapabilities {
    streaming: true,
    json_mode: true,
    auth: LlmAuthMode::Required,
    base_url: LlmBaseUrl::Fixed,
    model_source: ModelSource::Allowlist,
    context_window_tokens: Some(1_000_000),
    reasoning: false,
    tool_calling: false,
};

pub const LLM_CAPS_OPENAI: LlmCapabilities = LlmCapabilities {
    streaming: true,
    json_mode: true,
    auth: LlmAuthMode::Required,
    base_url: LlmBaseUrl::Fixed,
    model_source: ModelSource::Allowlist,
    context_window_tokens: Some(128_000),
    reasoning: false,
    tool_calling: false,
};

/// OpenAI-compatible servers (custom profiles): most support streaming
/// and JSON mode. Servers that do not are caught by the mandatory test-call at
/// profile save and by tolerant parse plus honest-decline at runtime.
///
/// Auth is optional (local servers often need no key); base URL and model are
/// user-supplied (Configurable / Freeform).
pub const LLM_CAPS_COMPATIBLE: LlmCapabilities = LlmCapabilities {
    streaming: true,
    json_mode: true,
    auth: LlmAuthMode::Optional,
    base_url: LlmBaseUrl::Configurable,
    model_source: ModelSource::Freeform,
    context_window_tokens: None,
    reasoning: false,
    tool_calling: false,
};

/// Chat-LLM capabilities per built-in provider. `None` when the provider has
/// no chat LLM at all (Soniox — mirrors `supports_native_summary: false`).
pub fn llm_capabilities_for(provider: AiProvider) -> Option<&'static LlmCapabilities> {
    match provider {
        AiProvider::Gemini => Some(&LLM_CAPS_GEMINI),
        AiProvider::OpenAi => Some(&LLM_CAPS_OPENAI),
        AiProvider::Soniox => None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalog {
    pub provider: AiProvider,
    pub languages: Vec<LanguageInfo>,
    pub live_models: Vec<AiModelInfo>,
    pub summary_models: Vec<AiModelInfo>,
    pub defaults: AiCatalogDefaults,
    pub capabilities: ProviderCapabilities,
    /// When set, Notes mode uses this fixed STT model instead of `live_models` (OpenAI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_stt_model: Option<AiModelInfo>,
}

pub fn get_provider_catalog(provider: AiProvider) -> ProviderCatalog {
    match provider {
        AiProvider::Gemini => ProviderCatalog {
            provider,
            languages: catalog_gemini_languages(),
            live_models: catalog_gemini_live_models(),
            summary_models: catalog_gemini_summary_models(),
            defaults: AiCatalogDefaults {
                live_model: default_gemini_live_model(),
                summary_model: default_gemini_summary_model(),
            },
            capabilities: ProviderCapabilities {
                supports_vad_config: true,
                supports_echo_target_language: true,
                live_upload_sample_rate: UPLOAD_SAMPLE_RATE,
                supports_native_summary: true,
                uses_separate_tts: false,
                bridge_emits_playback_audio: true,
                // Gemini Live Translate always includes translationConfig + AUDIO modalities.
                supports_notes_stt_only: false,
            },
            notes_stt_model: None,
        },
        AiProvider::OpenAi => ProviderCatalog {
            provider,
            languages: catalog_openai_languages(),
            live_models: catalog_openai_live_models(),
            summary_models: catalog_openai_summary_models(),
            defaults: AiCatalogDefaults {
                live_model: default_openai_live_model(),
                summary_model: default_openai_summary_model(),
            },
            capabilities: ProviderCapabilities {
                supports_vad_config: false,
                supports_echo_target_language: false,
                live_upload_sample_rate: OPENAI_UPLOAD_SAMPLE_RATE,
                supports_native_summary: true,
                uses_separate_tts: false,
                bridge_emits_playback_audio: true,
                // Notes uses Realtime transcription session (not translations WS).
                supports_notes_stt_only: true,
            },
            notes_stt_model: Some(catalog_openai_notes_stt_model()),
        },
        AiProvider::Soniox => ProviderCatalog {
            provider,
            languages: catalog_soniox_languages(),
            live_models: catalog_soniox_live_models(),
            summary_models: catalog_soniox_summary_models(),
            defaults: AiCatalogDefaults {
                live_model: default_soniox_live_model(),
                summary_model: default_soniox_summary_model(),
            },
            capabilities: ProviderCapabilities {
                supports_vad_config: false,
                supports_echo_target_language: false,
                live_upload_sample_rate: SONIOX_UPLOAD_SAMPLE_RATE,
                supports_native_summary: false,
                uses_separate_tts: true,
                bridge_emits_playback_audio: false,
                // STT config can omit `translation` one_way for Notes.
                supports_notes_stt_only: true,
            },
            notes_stt_model: None,
        },
    }
}

pub fn is_supported_language_for_provider(provider: AiProvider, code: &str) -> bool {
    match provider {
        AiProvider::Gemini => is_gemini_supported_language(code),
        AiProvider::OpenAi => is_openai_supported_language(code),
        AiProvider::Soniox => is_soniox_supported_language(code),
    }
}

pub fn normalize_live_model_for_provider(provider: AiProvider, model: &str) -> String {
    match provider {
        AiProvider::Gemini => normalize_gemini_live_model(model),
        AiProvider::OpenAi => normalize_openai_live_model(model),
        AiProvider::Soniox => normalize_soniox_live_model(model),
    }
}

pub fn normalize_summary_model_for_provider(provider: AiProvider, model: &str) -> String {
    match provider {
        AiProvider::Gemini => normalize_gemini_summary_model(model),
        AiProvider::OpenAi => normalize_openai_summary_model(model),
        AiProvider::Soniox => normalize_soniox_summary_model(model),
    }
}

pub fn default_live_model_for_provider(provider: AiProvider) -> String {
    get_provider_catalog(provider).defaults.live_model
}

pub fn default_summary_model_for_provider(provider: AiProvider) -> String {
    get_provider_catalog(provider).defaults.summary_model
}

pub fn catalog_supported_languages_for_provider(provider: AiProvider) -> Vec<LanguageInfo> {
    get_provider_catalog(provider).languages
}

/// Seed persisted live-model catalog from static provider definitions.
/// Each model copies the provider language list (Gemini ≠ OpenAI ≠ Soniox).
pub fn seed_live_models(provider: AiProvider) -> Vec<LiveModelOption> {
    let catalog = get_provider_catalog(provider);
    let languages = catalog.languages;
    catalog
        .live_models
        .into_iter()
        .map(|model| LiveModelOption {
            id: model.id,
            name: Some(model.label),
            languages: languages.clone(),
        })
        .collect()
}

/// Languages for a selected live model from a persisted catalog (fallback: provider static).
pub fn languages_for_live_model(
    provider: AiProvider,
    live_model: &str,
    catalog: &[LiveModelOption],
) -> Vec<LanguageInfo> {
    if let Some(model) = catalog.iter().find(|m| m.id == live_model) {
        if !model.languages.is_empty() {
            return model.languages.clone();
        }
    }
    catalog_supported_languages_for_provider(provider)
}

/// Clamp live model + languages to a persisted catalog when non-empty; otherwise static lists.
pub fn clamp_to_live_catalog(
    provider: AiProvider,
    catalog: &[LiveModelOption],
    live_model: &mut String,
    my_language: &mut String,
    meeting_language: &mut String,
) -> bool {
    let mut changed = false;
    if !catalog.is_empty() {
        if !catalog.iter().any(|m| m.id == *live_model) {
            *live_model = catalog
                .first()
                .map(|m| m.id.clone())
                .unwrap_or_else(|| default_live_model_for_provider(provider));
            changed = true;
        }
    } else {
        let normalized = normalize_live_model_for_provider(provider, live_model);
        if normalized != *live_model {
            *live_model = normalized;
            changed = true;
        }
    }

    let langs = languages_for_live_model(provider, live_model, catalog);
    if !langs.is_empty() {
        if !langs.iter().any(|l| l.code == *my_language) {
            *my_language = langs
                .iter()
                .find(|l| l.code == "vi")
                .or_else(|| langs.first())
                .map(|l| l.code.clone())
                .unwrap_or_else(|| "vi".to_string());
            changed = true;
        }
        if !langs.iter().any(|l| l.code == *meeting_language) {
            *meeting_language = langs
                .iter()
                .find(|l| l.code == "en")
                .or_else(|| langs.first())
                .map(|l| l.code.clone())
                .unwrap_or_else(|| "en".to_string());
            changed = true;
        }
    } else {
        if !is_supported_language_for_provider(provider, my_language) {
            *my_language = "vi".to_string();
            changed = true;
        }
        if !is_supported_language_for_provider(provider, meeting_language) {
            *meeting_language = "en".to_string();
            changed = true;
        }
    }
    changed
}

pub fn migrate_config_for_provider(
    provider: AiProvider,
    live_model: &mut String,
    summary_model: &mut String,
    my_language: &mut String,
    meeting_language: &mut String,
) -> bool {
    let catalog = get_provider_catalog(provider);
    let mut changed = false;

    if !catalog.live_models.iter().any(|m| m.id == *live_model) {
        *live_model = catalog.defaults.live_model.clone();
        changed = true;
    }
    if provider != AiProvider::Soniox
        && !catalog
            .summary_models
            .iter()
            .any(|m| m.id == *summary_model)
    {
        *summary_model = catalog.defaults.summary_model.clone();
        changed = true;
    }
    if !is_supported_language_for_provider(provider, my_language) {
        *my_language = "vi".to_string();
        changed = true;
    }
    if !is_supported_language_for_provider(provider, meeting_language) {
        *meeting_language = "en".to_string();
        changed = true;
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_capabilities_matrix() {
        let gemini = llm_capabilities_for(AiProvider::Gemini).expect("gemini caps");
        assert!(gemini.streaming && gemini.json_mode);
        assert_eq!(gemini.auth, LlmAuthMode::Required);
        assert_eq!(gemini.base_url, LlmBaseUrl::Fixed);
        assert_eq!(gemini.model_source, ModelSource::Allowlist);
        assert!(!gemini.reasoning && !gemini.tool_calling);

        let openai = llm_capabilities_for(AiProvider::OpenAi).expect("openai caps");
        assert!(openai.streaming && openai.json_mode);
        assert_eq!(openai.auth, LlmAuthMode::Required);

        assert!(llm_capabilities_for(AiProvider::Soniox).is_none());
    }

    #[test]
    fn llm_capabilities_serde_camel_case() {
        let caps = llm_capabilities_for(AiProvider::Gemini).expect("caps");
        let json = serde_json::to_value(caps).expect("serialize");
        assert_eq!(json["auth"], "required");
        assert_eq!(json["baseUrl"], "fixed");
        assert_eq!(json["modelSource"], "allowlist");
        assert_eq!(json["streaming"], true);
        let round: LlmCapabilities = serde_json::from_value(json).expect("deserialize");
        assert_eq!(&round, caps);
    }

    #[test]
    fn openai_notes_stt_model_is_whisper_not_in_live_catalog() {
        let openai = get_provider_catalog(AiProvider::OpenAi);
        let notes = openai.notes_stt_model.expect("OpenAI notes STT model");
        assert_eq!(notes.id, "gpt-realtime-whisper");
        assert!(!openai.live_models.iter().any(|m| m.id == notes.id));
        assert!(openai
            .live_models
            .iter()
            .any(|m| m.id == "gpt-realtime-translate"));
    }

    #[test]
    fn gemini_catalog_has_more_languages_than_openai() {
        let gemini = get_provider_catalog(AiProvider::Gemini);
        let openai = get_provider_catalog(AiProvider::OpenAi);
        assert!(gemini.languages.len() > openai.languages.len());
        assert_eq!(openai.languages.len(), 13);
    }

    #[test]
    fn seed_live_models_attach_provider_languages() {
        let gemini = seed_live_models(AiProvider::Gemini);
        let openai = seed_live_models(AiProvider::OpenAi);
        assert!(!gemini.is_empty());
        assert!(!openai.is_empty());
        assert_eq!(
            gemini[0].languages.len(),
            catalog_supported_languages_for_provider(AiProvider::Gemini).len()
        );
        assert_eq!(openai[0].languages.len(), 13);
        assert!(gemini[0].languages.len() > openai[0].languages.len());
    }

    #[test]
    fn clamp_to_persisted_catalog_resets_unknown_model_and_lang() {
        let catalog = vec![LiveModelOption {
            id: "stt-rt-v5".to_string(),
            name: Some("v5".to_string()),
            languages: vec![
                LanguageInfo {
                    code: "en".to_string(),
                    name: "English".to_string(),
                    country_code: "US".to_string(),
                },
                LanguageInfo {
                    code: "vi".to_string(),
                    name: "Vietnamese".to_string(),
                    country_code: "VN".to_string(),
                },
            ],
        }];
        let mut live = "bogus".to_string();
        let mut my = "ar".to_string();
        let mut meeting = "fr".to_string();
        assert!(clamp_to_live_catalog(
            AiProvider::Soniox,
            &catalog,
            &mut live,
            &mut my,
            &mut meeting,
        ));
        assert_eq!(live, "stt-rt-v5");
        assert_eq!(my, "vi");
        assert_eq!(meeting, "en");
    }

    #[test]
    fn notes_stt_only_capability_per_provider() {
        assert!(
            !get_provider_catalog(AiProvider::Gemini)
                .capabilities
                .supports_notes_stt_only
        );
        assert!(
            get_provider_catalog(AiProvider::OpenAi)
                .capabilities
                .supports_notes_stt_only
        );
        assert!(
            get_provider_catalog(AiProvider::Soniox)
                .capabilities
                .supports_notes_stt_only
        );
    }

    #[test]
    fn soniox_catalog_uses_separate_tts() {
        let soniox = get_provider_catalog(AiProvider::Soniox);
        assert!(soniox.capabilities.uses_separate_tts);
        assert!(!soniox.capabilities.bridge_emits_playback_audio);
        assert!(!soniox.capabilities.supports_native_summary);
        assert_eq!(soniox.defaults.live_model, "stt-rt-v5");
    }

    #[test]
    fn bridge_emits_playback_audio_per_provider() {
        assert!(
            get_provider_catalog(AiProvider::Gemini)
                .capabilities
                .bridge_emits_playback_audio
        );
        assert!(
            get_provider_catalog(AiProvider::OpenAi)
                .capabilities
                .bridge_emits_playback_audio
        );
        assert!(
            !get_provider_catalog(AiProvider::Soniox)
                .capabilities
                .bridge_emits_playback_audio
        );
    }

    #[test]
    fn migrate_openai_resets_unsupported_arabic() {
        let mut live = "gemini-3.5-live-translate-preview".to_string();
        let mut summary = "gemini-2.5-flash".to_string();
        let mut my = "ar".to_string();
        let mut meeting = "en".to_string();
        assert!(migrate_config_for_provider(
            AiProvider::OpenAi,
            &mut live,
            &mut summary,
            &mut my,
            &mut meeting,
        ));
        assert_eq!(live, default_openai_live_model());
        assert_eq!(my, "vi");
    }

    #[test]
    fn is_supported_language_matrix() {
        assert!(is_supported_language_for_provider(AiProvider::Gemini, "vi"));
        assert!(is_supported_language_for_provider(AiProvider::Gemini, "ar"));
        assert!(is_supported_language_for_provider(AiProvider::OpenAi, "en"));
        assert!(!is_supported_language_for_provider(
            AiProvider::OpenAi,
            "ar"
        ));
        assert!(is_supported_language_for_provider(AiProvider::Soniox, "vi"));
    }

    #[test]
    fn normalize_unknown_models_to_defaults() {
        assert_eq!(
            normalize_live_model_for_provider(AiProvider::Gemini, "not-a-model"),
            default_live_model_for_provider(AiProvider::Gemini)
        );
        assert_eq!(
            normalize_summary_model_for_provider(AiProvider::OpenAi, "not-a-model"),
            default_summary_model_for_provider(AiProvider::OpenAi)
        );
        assert_eq!(
            normalize_live_model_for_provider(AiProvider::Soniox, "stt-rt-v5"),
            "stt-rt-v5"
        );
    }

    #[test]
    fn migrate_soniox_keeps_summary_resets_invalid_live_and_meeting_lang() {
        let mut live = "bogus-live".to_string();
        let mut summary = "custom-summary-keep".to_string();
        let mut my = "vi".to_string();
        let mut meeting = "xx".to_string();
        assert!(migrate_config_for_provider(
            AiProvider::Soniox,
            &mut live,
            &mut summary,
            &mut my,
            &mut meeting,
        ));
        assert_eq!(live, default_soniox_live_model());
        assert_eq!(summary, "custom-summary-keep");
        assert_eq!(meeting, "en");
    }
}
