use crate::ai::types::{AiCatalog, AiModelInfo, LanguageInfo};

pub const GEMINI_REST_BASE: &str = "https://generativelanguage.googleapis.com";
pub const GEMINI_API_VERSION: &str = "v1beta";
pub const GEMINI_REST_TIMEOUT_SECS: u64 = 120;
pub const UPLOAD_SAMPLE_RATE: u32 = 16000;

pub const DEFAULT_LIVE_MODEL: &str = "gemini-3.5-live-translate-preview";
pub const DEFAULT_SUMMARY_MODEL: &str = "gemini-2.5-flash";

pub const ALLOWED_LIVE_MODELS: &[&str] = &["gemini-3.5-live-translate-preview"];

pub const ALLOWED_SUMMARY_MODELS: &[&str] = &["gemini-2.5-flash", "gemini-2.0-flash"];

pub struct SupportedLanguage {
    pub code: &'static str,
    pub name: &'static str,
}

pub const SUPPORTED_LANGUAGES: &[SupportedLanguage] = &[
    SupportedLanguage {
        code: "en",
        name: "English",
    },
    SupportedLanguage {
        code: "vi",
        name: "Vietnamese",
    },
    SupportedLanguage {
        code: "ja",
        name: "Japanese",
    },
    SupportedLanguage {
        code: "ko",
        name: "Korean",
    },
    SupportedLanguage {
        code: "zh",
        name: "Chinese",
    },
    SupportedLanguage {
        code: "fr",
        name: "French",
    },
    SupportedLanguage {
        code: "de",
        name: "German",
    },
    SupportedLanguage {
        code: "es",
        name: "Spanish",
    },
    SupportedLanguage {
        code: "pt",
        name: "Portuguese",
    },
    SupportedLanguage {
        code: "it",
        name: "Italian",
    },
    SupportedLanguage {
        code: "ru",
        name: "Russian",
    },
    SupportedLanguage {
        code: "ar",
        name: "Arabic",
    },
    SupportedLanguage {
        code: "hi",
        name: "Hindi",
    },
    SupportedLanguage {
        code: "th",
        name: "Thai",
    },
    SupportedLanguage {
        code: "id",
        name: "Indonesian",
    },
    SupportedLanguage {
        code: "nl",
        name: "Dutch",
    },
    SupportedLanguage {
        code: "pl",
        name: "Polish",
    },
    SupportedLanguage {
        code: "tr",
        name: "Turkish",
    },
];

pub fn default_gemini_live_model() -> String {
    DEFAULT_LIVE_MODEL.to_string()
}

pub fn default_gemini_summary_model() -> String {
    DEFAULT_SUMMARY_MODEL.to_string()
}

pub fn normalize_gemini_live_model(model: &str) -> String {
    let trimmed = model.trim();
    if ALLOWED_LIVE_MODELS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        default_gemini_live_model()
    }
}

pub fn normalize_gemini_summary_model(model: &str) -> String {
    let trimmed = model.trim();
    if ALLOWED_SUMMARY_MODELS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        default_gemini_summary_model()
    }
}

pub fn is_gemini_supported_language(code: &str) -> bool {
    SUPPORTED_LANGUAGES.iter().any(|lang| lang.code == code)
}

pub fn catalog_gemini_languages() -> Vec<LanguageInfo> {
    SUPPORTED_LANGUAGES
        .iter()
        .map(|lang| LanguageInfo::new(lang.code, lang.name))
        .collect()
}

pub fn catalog_gemini_live_models() -> Vec<AiModelInfo> {
    vec![AiModelInfo {
        id: DEFAULT_LIVE_MODEL.to_string(),
        label: "Gemini 3.5 Live Translate".to_string(),
        description: "Real-time speech translation over Gemini Live WebSocket.".to_string(),
    }]
}

pub fn catalog_gemini_summary_models() -> Vec<AiModelInfo> {
    vec![
        AiModelInfo {
            id: "gemini-2.5-flash".to_string(),
            label: "Gemini 2.5 Flash".to_string(),
            description: "Fast REST summarization with JSON output.".to_string(),
        },
        AiModelInfo {
            id: "gemini-2.0-flash".to_string(),
            label: "Gemini 2.0 Flash".to_string(),
            description: "Stable flash model for meeting summaries.".to_string(),
        },
    ]
}

pub fn ai_catalog() -> AiCatalog {
    let catalog =
        crate::capabilities::get_provider_catalog(crate::ai::provider::AiProvider::Gemini);
    AiCatalog {
        languages: catalog.languages,
        live_models: catalog.live_models,
        summary_models: catalog.summary_models,
        defaults: catalog.defaults,
    }
}

pub fn live_ws_url() -> String {
    format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.{GEMINI_API_VERSION}.GenerativeService.BidiGenerateContent"
    )
}

pub fn rest_generate_content_url(model: &str) -> String {
    format!("{GEMINI_REST_BASE}/{GEMINI_API_VERSION}/models/{model}:generateContent")
}

pub fn rest_stream_generate_content_url(model: &str) -> String {
    format!("{GEMINI_REST_BASE}/{GEMINI_API_VERSION}/models/{model}:streamGenerateContent?alt=sse")
}

/// HTTP / WebSocket header name for Gemini API keys (never put the key in the URL).
pub const GOOGLE_API_KEY_HEADER: &str = "x-goog-api-key";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_unknown_models() {
        assert_eq!(normalize_gemini_live_model("bad"), DEFAULT_LIVE_MODEL);
        assert_eq!(normalize_gemini_summary_model("bad"), DEFAULT_SUMMARY_MODEL);
    }

    #[test]
    fn builds_rest_url_without_api_key() {
        let url = rest_generate_content_url(DEFAULT_SUMMARY_MODEL);
        assert!(url.contains("generateContent"));
        assert!(url.contains(DEFAULT_SUMMARY_MODEL));
        assert!(!url.contains("key="));
        assert!(!url.contains("secret-key-value"));
    }

    #[test]
    fn builds_stream_url_with_sse_and_without_api_key() {
        let url = rest_stream_generate_content_url(DEFAULT_SUMMARY_MODEL);
        assert!(url.contains("streamGenerateContent"));
        assert!(url.contains("alt=sse"));
        assert!(url.contains(DEFAULT_SUMMARY_MODEL));
        assert!(!url.contains("key="));
    }

    #[test]
    fn builds_live_ws_url_without_api_key() {
        let url = live_ws_url();
        assert!(url.contains("BidiGenerateContent"));
        assert!(!url.contains('?'));
        assert!(!url.contains("key="));
    }
}
