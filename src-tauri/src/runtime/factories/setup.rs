//! Provider-specific live connect setup (typed payloads; no Soniox bag on STS).

use crate::ai::AiProvider;
use crate::config::AppConfig;
use crate::providers::shared::live::{LiveSetupOptions, SonioxTranslationTerm};

pub use crate::providers::shared::live::{SonioxLiveSetup, StsLiveSetup};

/// Setup payload built for a specific live provider.
#[derive(Debug, Clone)]
pub enum LiveConnectSetup {
    Gemini(StsLiveSetup),
    OpenAi(StsLiveSetup),
    Soniox(SonioxLiveSetup),
}

impl LiveConnectSetup {
    pub fn from_config(config: &AppConfig) -> Self {
        match config.ai_provider {
            AiProvider::Gemini => Self::Gemini(sts_live_setup_from_config(config)),
            AiProvider::OpenAi => Self::OpenAi(sts_live_setup_from_config(config)),
            AiProvider::Soniox => Self::Soniox(soniox_live_setup_from_config(config)),
        }
    }

    /// Flatten to [`LiveSetupOptions`] for existing provider protocol builders.
    pub fn into_options(self) -> LiveSetupOptions {
        match self {
            Self::Gemini(sts) | Self::OpenAi(sts) => sts.into(),
            Self::Soniox(soniox) => soniox.into(),
        }
    }
}

fn sts_live_setup_from_config(config: &AppConfig) -> StsLiveSetup {
    StsLiveSetup {
        model: config.live_model.clone(),
        echo_target_language: config.echo_target_language,
        vad_silence_duration_ms: config.vad_silence_duration_ms,
        vad_start_sensitivity: config.vad_start_sensitivity.to_api_value().to_string(),
        vad_end_sensitivity: config.vad_end_sensitivity.to_end_api_value().to_string(),
        translation_enabled: config.session_mode.translation_enabled(),
    }
}

fn soniox_live_setup_from_config(config: &AppConfig) -> SonioxLiveSetup {
    let mut hints = Vec::new();
    if !config.my_language.trim().is_empty() {
        hints.push(config.my_language.clone());
    }
    if !config.meeting_language.trim().is_empty() && config.meeting_language != config.my_language {
        hints.push(config.meeting_language.clone());
    }
    let resolved = crate::providers::soniox::resolve_active_soniox_context(
        &config.soniox.soniox_always_on,
        &config.soniox.soniox_context_profiles,
        config.soniox.soniox_active_context_profile_id.as_deref(),
    );
    let translation_terms: Vec<SonioxTranslationTerm> = resolved
        .translation_terms
        .into_iter()
        .map(|(source, target)| SonioxTranslationTerm { source, target })
        .collect();
    SonioxLiveSetup {
        model: config.live_model.clone(),
        language_hints: hints,
        soniox_general: resolved.general,
        soniox_context_text: resolved.text,
        soniox_glossary_terms: resolved.terms,
        soniox_translation_terms: translation_terms,
        soniox_endpoint_latency_adjustment_level: config
            .soniox
            .soniox_endpoint_latency_adjustment_level,
        soniox_endpoint_sensitivity: config.soniox.soniox_endpoint_sensitivity,
        soniox_max_endpoint_delay_ms: config.soniox.soniox_max_endpoint_delay_ms,
        translation_enabled: config.session_mode.translation_enabled(),
    }
}

/// Build setup options via typed [`LiveConnectSetup`], then flatten for bridges.
pub fn live_setup_for_provider(config: &AppConfig) -> LiveSetupOptions {
    LiveConnectSetup::from_config(config).into_options()
}
