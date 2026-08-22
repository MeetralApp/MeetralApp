use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxTranslationTerm {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxGeneralPair {
    pub key: String,
    pub value: String,
}

/// Four Soniox context sections — shared by Always-on and each per-meeting profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxContextPayload {
    #[serde(default)]
    pub general: Vec<SonioxGeneralPair>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub terms: Vec<String>,
    #[serde(default)]
    pub translation_terms: Vec<SonioxTranslationTerm>,
}

impl SonioxContextPayload {
    pub fn is_empty(&self) -> bool {
        self.general
            .iter()
            .all(|p| p.key.trim().is_empty() && p.value.trim().is_empty())
            && self.text.trim().is_empty()
            && self.terms.iter().all(|t| t.trim().is_empty())
            && self
                .translation_terms
                .iter()
                .all(|t| t.source.trim().is_empty() && t.target.trim().is_empty())
    }
}

fn default_include_always_on() -> bool {
    true
}

/// Per-meeting context template (not a SQLite meeting row).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SonioxContextProfile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_include_always_on")]
    pub include_always_on: bool,
    #[serde(default)]
    pub payload: SonioxContextPayload,
}

impl SonioxContextProfile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            include_always_on: true,
            payload: SonioxContextPayload::default(),
        }
    }
}

/// STS (Gemini / OpenAI) live session setup — no Soniox fields.
#[derive(Debug, Clone)]
pub struct StsLiveSetup {
    pub model: String,
    pub echo_target_language: bool,
    pub vad_silence_duration_ms: u32,
    pub vad_start_sensitivity: String,
    pub vad_end_sensitivity: String,
    /// OpenAI Notes: false → Realtime transcription session (no MT).
    pub translation_enabled: bool,
}

/// Soniox live session setup — no STS VAD/echo fields.
#[derive(Debug, Clone)]
pub struct SonioxLiveSetup {
    pub model: String,
    pub language_hints: Vec<String>,
    pub soniox_general: Vec<SonioxGeneralPair>,
    pub soniox_context_text: String,
    pub soniox_glossary_terms: Vec<String>,
    pub soniox_translation_terms: Vec<SonioxTranslationTerm>,
    pub soniox_endpoint_latency_adjustment_level: u8,
    pub soniox_endpoint_sensitivity: f64,
    pub soniox_max_endpoint_delay_ms: u32,
    pub translation_enabled: bool,
}

#[derive(Debug, Clone)]
/// Flat setup bag for provider protocol builders (built via typed → `into_options`).
/// VAD/echo apply to Gemini; Soniox fields apply to Soniox only.
pub struct LiveSetupOptions {
    pub model: String,
    pub echo_target_language: bool,
    pub vad_silence_duration_ms: u32,
    pub vad_start_sensitivity: String,
    pub vad_end_sensitivity: String,
    /// Soniox-only extras (ignored by Gemini/OpenAI).
    pub language_hints: Vec<String>,
    /// Structured `context.general` key-value pairs (resolved for this session).
    pub soniox_general: Vec<SonioxGeneralPair>,
    /// Free-form `context.text` background (resolved).
    pub soniox_context_text: String,
    pub soniox_glossary_terms: Vec<String>,
    pub soniox_translation_terms: Vec<SonioxTranslationTerm>,
    pub soniox_endpoint_latency_adjustment_level: u8,
    pub soniox_endpoint_sensitivity: f64,
    pub soniox_max_endpoint_delay_ms: u32,
    /// When false, Soniox omits MT; OpenAI uses Realtime transcription; Gemini Notes blocked.
    pub translation_enabled: bool,
}

impl From<StsLiveSetup> for LiveSetupOptions {
    fn from(sts: StsLiveSetup) -> Self {
        let defaults = Self::default();
        Self {
            model: sts.model,
            echo_target_language: sts.echo_target_language,
            vad_silence_duration_ms: sts.vad_silence_duration_ms,
            vad_start_sensitivity: sts.vad_start_sensitivity,
            vad_end_sensitivity: sts.vad_end_sensitivity,
            language_hints: Vec::new(),
            soniox_general: Vec::new(),
            soniox_context_text: String::new(),
            soniox_glossary_terms: Vec::new(),
            soniox_translation_terms: Vec::new(),
            soniox_endpoint_latency_adjustment_level: defaults
                .soniox_endpoint_latency_adjustment_level,
            soniox_endpoint_sensitivity: defaults.soniox_endpoint_sensitivity,
            soniox_max_endpoint_delay_ms: defaults.soniox_max_endpoint_delay_ms,
            translation_enabled: sts.translation_enabled,
        }
    }
}

impl From<SonioxLiveSetup> for LiveSetupOptions {
    fn from(soniox: SonioxLiveSetup) -> Self {
        let defaults = Self::default();
        Self {
            model: soniox.model,
            echo_target_language: defaults.echo_target_language,
            vad_silence_duration_ms: defaults.vad_silence_duration_ms,
            vad_start_sensitivity: defaults.vad_start_sensitivity,
            vad_end_sensitivity: defaults.vad_end_sensitivity,
            language_hints: soniox.language_hints,
            soniox_general: soniox.soniox_general,
            soniox_context_text: soniox.soniox_context_text,
            soniox_glossary_terms: soniox.soniox_glossary_terms,
            soniox_translation_terms: soniox.soniox_translation_terms,
            soniox_endpoint_latency_adjustment_level: soniox
                .soniox_endpoint_latency_adjustment_level,
            soniox_endpoint_sensitivity: soniox.soniox_endpoint_sensitivity,
            soniox_max_endpoint_delay_ms: soniox.soniox_max_endpoint_delay_ms,
            translation_enabled: soniox.translation_enabled,
        }
    }
}

impl Default for LiveSetupOptions {
    fn default() -> Self {
        Self {
            model: crate::providers::gemini::config::DEFAULT_LIVE_MODEL.to_string(),
            echo_target_language: true,
            vad_silence_duration_ms: 800,
            vad_start_sensitivity: "START_SENSITIVITY_LOW".to_string(),
            vad_end_sensitivity: "END_SENSITIVITY_LOW".to_string(),
            language_hints: Vec::new(),
            soniox_general: Vec::new(),
            soniox_context_text: String::new(),
            soniox_glossary_terms: Vec::new(),
            soniox_translation_terms: Vec::new(),
            soniox_endpoint_latency_adjustment_level:
                crate::providers::soniox::config::ENDPOINT_LATENCY_ADJUSTMENT_LEVEL,
            soniox_endpoint_sensitivity: crate::providers::soniox::config::ENDPOINT_SENSITIVITY,
            soniox_max_endpoint_delay_ms: crate::providers::soniox::config::MAX_ENDPOINT_DELAY_MS,
            translation_enabled: true,
        }
    }
}
