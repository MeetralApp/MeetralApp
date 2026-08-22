use crate::ai::types::{AiModelInfo, LanguageInfo};

pub const OPENAI_REST_BASE: &str = "https://api.openai.com";
pub const OPENAI_TRANSLATIONS_WS: &str = "wss://api.openai.com/v1/realtime/translations";
pub const OPENAI_REALTIME_WS: &str = "wss://api.openai.com/v1/realtime";
pub const UPLOAD_SAMPLE_RATE: u32 = 24000;

/// Whisper model id for Realtime transcription sessions (Notes STT-only).
pub const OPENAI_TRANSCRIPTION_MODEL: &str = "gpt-realtime-whisper";

pub const DEFAULT_OPENAI_LIVE_MODEL: &str = "gpt-realtime-translate";
pub const DEFAULT_OPENAI_SUMMARY_MODEL: &str = "gpt-4o";

pub const ALLOWED_OPENAI_LIVE_MODELS: &[&str] = &["gpt-realtime-translate"];

pub const ALLOWED_OPENAI_SUMMARY_MODELS: &[&str] = &["gpt-4o", "gpt-4.1-mini"];

pub struct OpenAiLanguage {
    pub code: &'static str,
    pub name: &'static str,
}

/// OpenAI realtime translate supports 13 output languages.
pub const OPENAI_OUTPUT_LANGUAGES: &[OpenAiLanguage] = &[
    OpenAiLanguage {
        code: "en",
        name: "English",
    },
    OpenAiLanguage {
        code: "es",
        name: "Spanish",
    },
    OpenAiLanguage {
        code: "pt",
        name: "Portuguese",
    },
    OpenAiLanguage {
        code: "fr",
        name: "French",
    },
    OpenAiLanguage {
        code: "ja",
        name: "Japanese",
    },
    OpenAiLanguage {
        code: "ru",
        name: "Russian",
    },
    OpenAiLanguage {
        code: "zh",
        name: "Chinese",
    },
    OpenAiLanguage {
        code: "de",
        name: "German",
    },
    OpenAiLanguage {
        code: "ko",
        name: "Korean",
    },
    OpenAiLanguage {
        code: "hi",
        name: "Hindi",
    },
    OpenAiLanguage {
        code: "id",
        name: "Indonesian",
    },
    OpenAiLanguage {
        code: "vi",
        name: "Vietnamese",
    },
    OpenAiLanguage {
        code: "it",
        name: "Italian",
    },
];

pub fn default_openai_live_model() -> String {
    DEFAULT_OPENAI_LIVE_MODEL.to_string()
}

pub fn default_openai_summary_model() -> String {
    DEFAULT_OPENAI_SUMMARY_MODEL.to_string()
}

pub fn normalize_openai_live_model(model: &str) -> String {
    let trimmed = model.trim();
    if ALLOWED_OPENAI_LIVE_MODELS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        default_openai_live_model()
    }
}

pub fn normalize_openai_summary_model(model: &str) -> String {
    let trimmed = model.trim();
    if ALLOWED_OPENAI_SUMMARY_MODELS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        default_openai_summary_model()
    }
}

pub fn is_openai_supported_language(code: &str) -> bool {
    OPENAI_OUTPUT_LANGUAGES.iter().any(|lang| lang.code == code)
}

pub fn catalog_openai_languages() -> Vec<LanguageInfo> {
    OPENAI_OUTPUT_LANGUAGES
        .iter()
        .map(|lang| LanguageInfo::new(lang.code, lang.name))
        .collect()
}

pub fn catalog_openai_live_models() -> Vec<AiModelInfo> {
    vec![AiModelInfo {
        id: DEFAULT_OPENAI_LIVE_MODEL.to_string(),
        label: "GPT Realtime Translate".to_string(),
        description: "OpenAI live speech translation over WebSocket.".to_string(),
    }]
}

/// Fixed Notes STT model (not selectable as Interpreter live model).
pub fn catalog_openai_notes_stt_model() -> AiModelInfo {
    AiModelInfo {
        id: OPENAI_TRANSCRIPTION_MODEL.to_string(),
        label: "GPT Realtime Whisper".to_string(),
        description: "OpenAI Realtime transcription for Notes (STT-only; no translation session)."
            .to_string(),
    }
}

pub fn catalog_openai_summary_models() -> Vec<AiModelInfo> {
    vec![
        AiModelInfo {
            id: "gpt-4o".to_string(),
            label: "GPT-4o".to_string(),
            description: "High-quality meeting summaries with JSON output.".to_string(),
        },
        AiModelInfo {
            id: "gpt-4.1-mini".to_string(),
            label: "GPT-4.1 Mini".to_string(),
            description: "Faster summaries for long transcripts.".to_string(),
        },
    ]
}

pub fn translations_ws_url(model: &str) -> String {
    format!("{OPENAI_TRANSLATIONS_WS}?model={model}")
}

/// General Realtime WebSocket for transcription sessions (Notes).
/// Do **not** put `model=` on the URL — OpenAI treats that as a conversation session model
/// and rejects transcription models. Use `intent=transcription` only; the STT model goes in
/// `session.update` → `audio.input.transcription.model`.
pub fn transcription_ws_url() -> String {
    format!("{OPENAI_REALTIME_WS}?intent=transcription")
}

pub fn chat_completions_url() -> String {
    format!("{OPENAI_REST_BASE}/v1/chat/completions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_language_count_is_thirteen() {
        assert_eq!(OPENAI_OUTPUT_LANGUAGES.len(), 13);
    }

    #[test]
    fn transcription_ws_url_uses_intent_not_model_query() {
        let url = transcription_ws_url();
        assert_eq!(url, "wss://api.openai.com/v1/realtime?intent=transcription");
        assert!(!url.contains("model="));
        assert!(!url.contains(OPENAI_TRANSCRIPTION_MODEL));
    }

    #[test]
    fn normalizes_unknown_openai_models() {
        assert_eq!(
            normalize_openai_live_model("bad"),
            DEFAULT_OPENAI_LIVE_MODEL
        );
        assert_eq!(
            normalize_openai_summary_model("bad"),
            DEFAULT_OPENAI_SUMMARY_MODEL
        );
    }
}
