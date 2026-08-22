use crate::ai::types::{AiModelInfo, LanguageInfo};

pub const SONIOX_STT_WS: &str = "wss://stt-rt.soniox.com/transcribe-websocket";
pub const UPLOAD_SAMPLE_RATE: u32 = 16000;

pub const DEFAULT_SONIOX_LIVE_MODEL: &str = "stt-rt-v5";
/// Soniox has no native summary LLM — catalog keeps a placeholder for migrate;
/// runtime summary falls back to Gemini/OpenAI when those keys exist.
pub const DEFAULT_SONIOX_SUMMARY_MODEL: &str = "gemini-2.5-flash";

pub const ALLOWED_SONIOX_LIVE_MODELS: &[&str] = &["stt-rt-v5"];

/// Balanced defaults: responsive `<end>` without the most aggressive cutoffs.
/// Docs ranges: level 0–3, sensitivity −1.0–1.0, max_delay 500–3000 (default 2000).
pub const ENDPOINT_LATENCY_ADJUSTMENT_LEVEL: u8 = 2;
pub const ENDPOINT_SENSITIVITY: f64 = 0.3;
pub const MAX_ENDPOINT_DELAY_MS: u32 = 1000;

pub fn normalize_endpoint_latency_level(level: u8) -> u8 {
    level.min(3)
}

pub fn normalize_endpoint_sensitivity(sensitivity: f64) -> f64 {
    sensitivity.clamp(-1.0, 1.0)
}

pub fn normalize_max_endpoint_delay_ms(ms: u32) -> u32 {
    ms.clamp(500, 3000)
}

pub struct SonioxLanguage {
    pub code: &'static str,
    pub name: &'static str,
}

/// Soniox supports 60+ languages; ship the same practical set as Gemini.
pub const SONIOX_LANGUAGES: &[SonioxLanguage] = &[
    SonioxLanguage {
        code: "en",
        name: "English",
    },
    SonioxLanguage {
        code: "vi",
        name: "Vietnamese",
    },
    SonioxLanguage {
        code: "ja",
        name: "Japanese",
    },
    SonioxLanguage {
        code: "ko",
        name: "Korean",
    },
    SonioxLanguage {
        code: "zh",
        name: "Chinese",
    },
    SonioxLanguage {
        code: "fr",
        name: "French",
    },
    SonioxLanguage {
        code: "de",
        name: "German",
    },
    SonioxLanguage {
        code: "es",
        name: "Spanish",
    },
    SonioxLanguage {
        code: "pt",
        name: "Portuguese",
    },
    SonioxLanguage {
        code: "it",
        name: "Italian",
    },
    SonioxLanguage {
        code: "ru",
        name: "Russian",
    },
    SonioxLanguage {
        code: "ar",
        name: "Arabic",
    },
    SonioxLanguage {
        code: "hi",
        name: "Hindi",
    },
    SonioxLanguage {
        code: "th",
        name: "Thai",
    },
    SonioxLanguage {
        code: "id",
        name: "Indonesian",
    },
    SonioxLanguage {
        code: "nl",
        name: "Dutch",
    },
    SonioxLanguage {
        code: "pl",
        name: "Polish",
    },
    SonioxLanguage {
        code: "tr",
        name: "Turkish",
    },
];

pub fn default_soniox_live_model() -> String {
    DEFAULT_SONIOX_LIVE_MODEL.to_string()
}

pub fn default_soniox_summary_model() -> String {
    DEFAULT_SONIOX_SUMMARY_MODEL.to_string()
}

pub fn normalize_soniox_live_model(model: &str) -> String {
    let trimmed = model.trim();
    if ALLOWED_SONIOX_LIVE_MODELS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        default_soniox_live_model()
    }
}

pub fn normalize_soniox_summary_model(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        default_soniox_summary_model()
    } else {
        trimmed.to_string()
    }
}

pub fn is_soniox_supported_language(code: &str) -> bool {
    SONIOX_LANGUAGES.iter().any(|lang| lang.code == code)
}

pub fn catalog_soniox_languages() -> Vec<LanguageInfo> {
    SONIOX_LANGUAGES
        .iter()
        .map(|lang| LanguageInfo::new(lang.code, lang.name))
        .collect()
}

pub fn catalog_soniox_live_models() -> Vec<AiModelInfo> {
    vec![AiModelInfo {
        id: DEFAULT_SONIOX_LIVE_MODEL.to_string(),
        label: "Soniox STT RT v5".to_string(),
        description: "Real-time speech-to-text translation over Soniox WebSocket.".to_string(),
    }]
}

pub fn catalog_soniox_summary_models() -> Vec<AiModelInfo> {
    // Placeholder — UI disables summary when no Gemini/OpenAI key.
    vec![AiModelInfo {
        id: DEFAULT_SONIOX_SUMMARY_MODEL.to_string(),
        label: "Summary via Gemini / OpenAI".to_string(),
        description: "Soniox has no summary LLM. Add a Gemini or OpenAI key to enable summaries."
            .to_string(),
    }]
}

pub fn stt_ws_url() -> &'static str {
    SONIOX_STT_WS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_unknown_live_model() {
        assert_eq!(
            normalize_soniox_live_model("bad"),
            DEFAULT_SONIOX_LIVE_MODEL
        );
    }

    #[test]
    fn supports_vi_and_en() {
        assert!(is_soniox_supported_language("vi"));
        assert!(is_soniox_supported_language("en"));
    }
}
