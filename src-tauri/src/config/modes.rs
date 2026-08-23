use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    #[default]
    Interpreter,
    Notes,
}

impl SessionMode {
    pub fn translation_enabled(self) -> bool {
        matches!(self, Self::Interpreter)
    }

    pub fn is_notes(self) -> bool {
        matches!(self, Self::Notes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioPathMode {
    Direct,
    Translate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum PipelineOutputMode {
    #[serde(alias = "voice")]
    #[default]
    Translated,
    OriginalAudio,
    TextOnly,
}

impl PipelineOutputMode {
    pub fn needs_playback(self) -> bool {
        matches!(self, Self::Translated | Self::OriginalAudio)
    }

    pub fn supports_hot_switch(self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OutboundVoiceOutput {
    #[default]
    ProviderNative,
    /// Custom voice path (ElevenLabs or Fish Audio catalog / generated / cloned voices).
    #[serde(rename = "custom")]
    Custom,
}

impl OutboundVoiceOutput {
    pub fn uses_custom_tts(self) -> bool {
        matches!(self, Self::Custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum InboundVoiceOutput {
    #[default]
    ProviderNative,
    #[serde(rename = "custom")]
    Custom,
}

impl InboundVoiceOutput {
    pub fn uses_custom_tts(self) -> bool {
        matches!(self, Self::Custom)
    }
}

/// Which custom-voice vendor backs a column on the Custom voice path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CustomVoiceVendor {
    #[default]
    ElevenLabs,
    FishAudio,
}

impl CustomVoiceVendor {
    pub fn as_log_label(self) -> &'static str {
        match self {
            Self::ElevenLabs => "elevenlabs",
            Self::FishAudio => "fishaudio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptLayout {
    #[default]
    SideBySide,
    Stacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum VadSensitivity {
    #[default]
    Low,
    Medium,
    High,
}

impl VadSensitivity {
    /// Gemini Live translate accepts only `*_LOW` and `*_HIGH` — not `*_MEDIUM`.
    /// UI "Normal" stores [`Medium`] and maps to responsive start + patient end-of-turn.
    pub fn to_api_value(self) -> &'static str {
        match self {
            Self::Low => "START_SENSITIVITY_LOW",
            Self::Medium => "START_SENSITIVITY_HIGH",
            Self::High => "START_SENSITIVITY_HIGH",
        }
    }

    pub fn to_end_api_value(self) -> &'static str {
        match self {
            Self::Low => "END_SENSITIVITY_LOW",
            Self::Medium => "END_SENSITIVITY_LOW",
            Self::High => "END_SENSITIVITY_HIGH",
        }
    }
}
