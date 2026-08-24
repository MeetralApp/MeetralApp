use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCustomLatencyEvent {
    pub phrase_started_ms: u64,
    pub first_translated_text_ms: u64,
    pub first_tts_audio_ms: u64,
    #[serde(default)]
    pub flush_ms: u64,
    #[serde(default)]
    pub flush_reason: String,
    pub translate_ms: i64,
    pub tts_ms: i64,
    pub total_ms: i64,
}

#[derive(Debug, Clone)]
pub enum VoiceTtsStatus {
    Ready,
    Degraded { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum VoiceTtsStatusPayload {
    Ready,
    Degraded { message: String },
}

impl From<VoiceTtsStatus> for VoiceTtsStatusPayload {
    fn from(status: VoiceTtsStatus) -> Self {
        match status {
            VoiceTtsStatus::Ready => Self::Ready,
            VoiceTtsStatus::Degraded { message } => Self::Degraded { message },
        }
    }
}
