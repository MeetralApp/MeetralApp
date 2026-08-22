use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub country_code: String,
}

impl LanguageInfo {
    pub fn new(code: impl Into<String>, name: impl Into<String>) -> Self {
        let code = code.into();
        let country_code = crate::ai::language_flags::country_code_for_language(&code);
        Self {
            code,
            name: name.into(),
            country_code,
        }
    }

    /// Ensure `country_code` is filled from the shared flag map when empty.
    pub fn with_resolved_flag(mut self) -> Self {
        self.country_code =
            crate::ai::language_flags::resolve_country_code(&self.code, &self.country_code);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub id: String,
    pub label: String,
    pub description: String,
}

/// Persisted live/STT model catalog entry (languages attached per model).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LiveModelOption {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub languages: Vec<LanguageInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCatalogDefaults {
    pub live_model: String,
    pub summary_model: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCatalog {
    pub languages: Vec<LanguageInfo>,
    pub live_models: Vec<AiModelInfo>,
    pub summary_models: Vec<AiModelInfo>,
    pub defaults: AiCatalogDefaults,
}
