use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AiProvider {
    #[default]
    Gemini,
    OpenAi,
    Soniox,
}

impl AiProvider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Gemini => "Gemini",
            Self::OpenAi => "OpenAI",
            Self::Soniox => "Soniox",
        }
    }

    /// Stable lowercase machine tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::OpenAi => "openai",
            Self::Soniox => "soniox",
        }
    }

    pub fn api_key_field_name(self) -> &'static str {
        match self {
            Self::Gemini => "gemini_api_key",
            Self::OpenAi => "openai_api_key",
            Self::Soniox => "soniox_api_key",
        }
    }

    pub fn keychain_account(self) -> &'static str {
        match self {
            Self::Gemini => "gemini_api_key",
            Self::OpenAi => "openai_api_key",
            Self::Soniox => "soniox_api_key",
        }
    }
}
