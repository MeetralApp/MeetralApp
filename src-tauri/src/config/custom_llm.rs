//! Custom OpenAI-compatible LLM profiles. Local servers (Ollama,
//! LM Studio, llama.cpp, vLLM) are DATA, not code: one profile record + the
//! single `providers/compatible` impl cover the whole ecosystem — no new
//! `AiProvider` variants.
//!
//! Boundary: profile validation + the selection resolver live here (config
//! zone); wire protocol lives in `providers/compatible`; construction
//! dispatch in `runtime/factories`.

use serde::{Deserialize, Serialize};

use crate::ai::AiProvider;
use crate::capabilities::{llm_capabilities_for, LlmCapabilities, LLM_CAPS_COMPATIBLE};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomLlmProfile {
    /// uuid — generated in the upsert command, stable across edits.
    pub id: String,
    /// Display name, e.g. "Ollama (local)".
    pub label: String,
    /// Server origin — normalized: trimmed, no trailing '/'.
    pub base_url: String,
    /// Freeform chat model id (ModelSource::Freeform — no allowlist clamp).
    pub chat_model: String,
    /// Whether the server accepts OpenAI `response_format: json_object`.
    /// Detected at save/test: false → prompt-only JSON at runtime.
    /// Defaults true for profiles saved before this field existed.
    #[serde(default = "default_true")]
    pub json_mode: bool,
}

impl CustomLlmProfile {
    /// DPAPI/keychain account for this profile's API key — ciphertext
    /// lives in config.json under `encrypted_custom_llm_keys`, plaintext
    /// never persists.
    pub fn keychain_account(&self) -> String {
        format!("custom_llm:{}", self.id)
    }

    /// Capability descriptor for this profile — `json_mode` may be
    /// false when OpenRouter/upstream rejected `response_format` at probe.
    pub fn llm_capabilities(&self) -> LlmCapabilities {
        let mut caps = LLM_CAPS_COMPATIBLE;
        caps.json_mode = self.json_mode;
        caps
    }
}

/// Normalize + validate a profile (single validation site — the upsert
/// command calls this; config load re-normalizes defensively).
pub fn normalize_custom_llm_profile(profile: &mut CustomLlmProfile) -> Result<(), String> {
    profile.id = profile.id.trim().to_string();
    profile.label = profile.label.trim().to_string();
    profile.base_url = profile.base_url.trim().trim_end_matches('/').to_string();
    profile.chat_model = profile.chat_model.trim().to_string();

    if profile.label.is_empty() {
        return Err("Profile name is required.".into());
    }
    if profile.base_url.is_empty() {
        return Err("Base URL is required (e.g. http://localhost:11434).".into());
    }
    if !profile.base_url.starts_with("http://") && !profile.base_url.starts_with("https://") {
        return Err("Base URL must start with http:// or https://".into());
    }
    if profile.chat_model.is_empty() {
        return Err("Chat model is required (e.g. qwen3:8b).".into());
    }
    Ok(())
}

/// Public profile shape for the FE — never carries the API key, only a
/// `apiKeyConfigured` flag (same pattern as built-in key fields).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomLlmProfileView {
    #[serde(flatten)]
    pub profile: CustomLlmProfile,
    pub api_key_configured: bool,
}

/// Resolved chat-LLM selection: built-in provider or custom profile,
/// credentials + model already resolved. Owned so command handlers can drop
/// the config lock before awaiting.
#[derive(Debug, Clone)]
pub enum LlmSelection {
    BuiltIn {
        provider: AiProvider,
        api_key: String,
        model: String,
    },
    Custom {
        profile: CustomLlmProfile,
        /// `None` when the profile has no key — compatible servers have
        /// `auth = Optional`, so the selection is still usable.
        api_key: Option<String>,
    },
}

impl LlmSelection {
    pub fn chat_model(&self) -> &str {
        match self {
            Self::BuiltIn { model, .. } => model,
            Self::Custom { profile, .. } => &profile.chat_model,
        }
    }

    pub fn api_key(&self) -> Option<&str> {
        match self {
            Self::BuiltIn { api_key, .. } => Some(api_key),
            Self::Custom { api_key, .. } => api_key.as_deref(),
        }
    }

    /// Capability descriptor — gates/FE degrade by capability, never by
    /// provider identity. Owned so custom profiles can flip `json_mode`
    /// after the save-time probe without a static table entry.
    pub fn capabilities(&self) -> LlmCapabilities {
        match self {
            // Only Gemini/OpenAI are selectable as built-in summary
            // providers (normalize_summary_provider) — both have caps.
            Self::BuiltIn { provider, .. } => llm_capabilities_for(*provider)
                .cloned()
                .unwrap_or(LLM_CAPS_COMPATIBLE),
            Self::Custom { profile, .. } => profile.llm_capabilities(),
        }
    }

    /// Log label matching `SummaryLlmClient::origin_label`.
    pub fn origin_label(&self) -> String {
        match self {
            Self::BuiltIn { provider, .. } => format!("{provider:?}"),
            Self::Custom { profile, .. } => format!("custom:{}", profile.id),
        }
    }

    /// Built-in provider behind this selection, if any — domain gates that
    /// are provider-specific (summary language allowlist) key off this.
    pub fn builtin_provider(&self) -> Option<AiProvider> {
        match self {
            Self::BuiltIn { provider, .. } => Some(*provider),
            Self::Custom { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> CustomLlmProfile {
        CustomLlmProfile {
            id: "p1".into(),
            label: " Ollama ".into(),
            base_url: " http://localhost:11434/ ".into(),
            chat_model: " qwen3:8b ".into(),
            json_mode: true,
        }
    }

    #[test]
    fn normalize_trims_and_strips_trailing_slash() {
        let mut p = profile();
        normalize_custom_llm_profile(&mut p).expect("valid");
        assert_eq!(p.label, "Ollama");
        assert_eq!(p.base_url, "http://localhost:11434");
        assert_eq!(p.chat_model, "qwen3:8b");
    }

    #[test]
    fn normalize_requires_http_scheme() {
        let mut p = profile();
        p.base_url = "localhost:11434".into();
        let err = normalize_custom_llm_profile(&mut p).expect_err("scheme required");
        assert!(err.contains("http://"));
    }

    #[test]
    fn normalize_requires_label_and_model() {
        let mut p = profile();
        p.label = "  ".into();
        assert!(normalize_custom_llm_profile(&mut p).is_err());
        let mut p = profile();
        p.chat_model = String::new();
        assert!(normalize_custom_llm_profile(&mut p).is_err());
    }

    #[test]
    fn keychain_account_format() {
        let p = profile();
        assert_eq!(p.keychain_account(), "custom_llm:p1");
    }

    #[test]
    fn selection_capabilities_and_models() {
        let builtin = LlmSelection::BuiltIn {
            provider: AiProvider::Gemini,
            api_key: "k".into(),
            model: "gemini-2.5-flash".into(),
        };
        assert_eq!(builtin.chat_model(), "gemini-2.5-flash");
        assert_eq!(builtin.api_key(), Some("k"));
        assert!(builtin.capabilities().streaming);
        assert_eq!(builtin.builtin_provider(), Some(AiProvider::Gemini));

        let mut normalized = profile();
        normalize_custom_llm_profile(&mut normalized).expect("valid");
        let custom = LlmSelection::Custom {
            profile: normalized,
            api_key: None,
        };
        assert_eq!(custom.chat_model(), "qwen3:8b");
        assert_eq!(custom.api_key(), None);
        assert_eq!(
            custom.capabilities().auth,
            crate::capabilities::LlmAuthMode::Optional
        );
        assert!(custom.capabilities().json_mode);
        assert_eq!(custom.builtin_provider(), None);
        assert_eq!(custom.origin_label(), "custom:p1");

        let mut degraded = profile();
        degraded.json_mode = false;
        normalize_custom_llm_profile(&mut degraded).expect("valid");
        let custom_no_json = LlmSelection::Custom {
            profile: degraded,
            api_key: None,
        };
        assert!(!custom_no_json.capabilities().json_mode);
    }

    #[test]
    fn json_mode_defaults_true_on_deserialize() {
        let raw = r#"{
            "id":"p1","label":"Ollama","baseUrl":"http://localhost:11434",
            "chatModel":"qwen3:8b"
        }"#;
        let p: CustomLlmProfile = serde_json::from_str(raw).expect("legacy profile");
        assert!(p.json_mode);
    }

    #[test]
    fn legacy_embedding_fields_are_ignored_on_deserialize() {
        let raw = r#"{
            "id":"p1","label":"Ollama","baseUrl":"http://localhost:11434",
            "chatModel":"qwen3:8b","embeddingModel":"nomic-embed-text","embeddingDims":768
        }"#;
        let p: CustomLlmProfile = serde_json::from_str(raw).expect("legacy profile");
        assert_eq!(p.chat_model, "qwen3:8b");
    }
}
