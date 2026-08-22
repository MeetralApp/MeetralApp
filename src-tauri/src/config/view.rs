use serde::{Deserialize, Serialize};

use crate::ai::AiProvider;

use super::app_config::{default_inbound_original_ducked_gain, default_true, AppConfig};
use super::device::DeviceRef;
use super::elevenlabs_public::ElevenLabsPublicSettings;
use super::modes::{
    InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode, SessionMode, ThemePreference,
    TranscriptLayout, VadSensitivity,
};
use super::overlay_settings::OverlaySettings;
use super::soniox_settings::SonioxSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigView {
    pub ai_provider: AiProvider,
    pub api_key_configured: bool,
    /// True when a Gemini key is stored (independent of active live engine).
    #[serde(default)]
    pub gemini_api_key_configured: bool,
    /// True when an OpenAI key is stored (independent of active live engine).
    #[serde(default)]
    pub openai_api_key_configured: bool,
    pub my_language: String,
    pub meeting_language: String,
    #[serde(default)]
    pub session_mode: SessionMode,
    #[serde(default)]
    pub interpreter_my_language: String,
    #[serde(default)]
    pub interpreter_meeting_language: String,
    #[serde(default)]
    pub notes_language: String,
    #[serde(default)]
    pub interpreter_outbound_mode: Option<PipelineOutputMode>,
    #[serde(default)]
    pub interpreter_inbound_mode: Option<PipelineOutputMode>,
    #[serde(default)]
    pub interpreter_outbound_voice_output: Option<OutboundVoiceOutput>,
    #[serde(default)]
    pub interpreter_inbound_voice_output: Option<InboundVoiceOutput>,
    pub user_mic: DeviceRef,
    pub teams_mic_feed: DeviceRef,
    pub meeting_capture: DeviceRef,
    pub local_playback: DeviceRef,
    pub outbound_mode: PipelineOutputMode,
    pub inbound_mode: PipelineOutputMode,
    pub live_model: String,
    #[serde(default)]
    pub gemini_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default)]
    pub openai_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default)]
    pub soniox_live_models: Vec<crate::ai::LiveModelOption>,
    pub summary_model: String,
    /// LLM used for meeting summaries (Gemini or OpenAI only).
    #[serde(default)]
    pub summary_provider: AiProvider,
    pub echo_target_language: bool,
    pub vad_silence_duration_ms: u32,
    pub vad_start_sensitivity: VadSensitivity,
    pub vad_end_sensitivity: VadSensitivity,
    #[serde(default = "default_true")]
    pub keep_direct_audio: bool,
    #[serde(default = "default_true")]
    pub inbound_original_under_translation: bool,
    #[serde(default = "default_inbound_original_ducked_gain")]
    pub inbound_original_ducked_gain: f32,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub theme_preference: ThemePreference,
    #[serde(default)]
    pub proactive_session_refresh: bool,
    /// FE: `saveMeetingAudio`
    #[serde(default, rename = "saveMeetingAudio")]
    pub record_meeting_audio: bool,
    /// FE: `saveMeetingAudioFolder` — empty = app default recordings dir.
    #[serde(default, rename = "saveMeetingAudioFolder")]
    pub meeting_audio_save_folder: String,
    #[serde(default)]
    pub transcript_layout: TranscriptLayout,
    #[serde(default)]
    pub overlay: OverlaySettings,
    #[serde(default)]
    pub outbound_voice_output: OutboundVoiceOutput,
    #[serde(default)]
    pub inbound_voice_output: InboundVoiceOutput,
    #[serde(default)]
    pub soniox_api_key_configured: bool,
    /// Soniox runtime/TTS settings — flat camelCase on the wire via flatten.
    #[serde(flatten)]
    pub soniox: SonioxSettings,
    /// True when live=Soniox and a Gemini or OpenAI key exists for summaries.
    #[serde(default)]
    pub summary_fallback_available: bool,
    pub elevenlabs_api_key_configured: bool,
    /// ElevenLabs public settings — flat camelCase on the wire via flatten.
    #[serde(flatten)]
    pub elevenlabs: ElevenLabsPublicSettings,
    #[serde(default = "default_true")]
    pub artifacts_enabled: bool,
    #[serde(default)]
    pub answer_language: String,
    /// App-level meeting context for summary prompts.
    #[serde(default)]
    pub meeting_context: super::meeting_context::MeetingContextPayload,
    /// Custom OpenAI-compatible LLM profiles — API keys never leave
    /// the backend; only `apiKeyConfigured` flags are exposed.
    #[serde(default)]
    pub custom_llm_profiles: Vec<crate::config::CustomLlmProfileView>,
    /// `None` = built-in provider; `Some(id)` = selected custom profile.
    #[serde(default)]
    pub summary_custom_profile_id: Option<String>,
}

impl From<&AppConfig> for ConfigView {
    fn from(config: &AppConfig) -> Self {
        Self {
            ai_provider: config.ai_provider,
            api_key_configured: config.is_api_key_configured(),
            gemini_api_key_configured: !config.gemini_api_key.trim().is_empty(),
            openai_api_key_configured: !config.openai_api_key.trim().is_empty(),
            my_language: config.my_language.clone(),
            meeting_language: config.meeting_language.clone(),
            session_mode: config.session_mode,
            interpreter_my_language: config.interpreter_my_language.clone(),
            interpreter_meeting_language: config.interpreter_meeting_language.clone(),
            notes_language: config.notes_language.clone(),
            interpreter_outbound_mode: config.interpreter_outbound_mode,
            interpreter_inbound_mode: config.interpreter_inbound_mode,
            interpreter_outbound_voice_output: config.interpreter_outbound_voice_output,
            interpreter_inbound_voice_output: config.interpreter_inbound_voice_output,
            user_mic: config.user_mic.clone(),
            teams_mic_feed: config.teams_mic_feed.clone(),
            meeting_capture: config.meeting_capture.clone(),
            local_playback: config.local_playback.clone(),
            outbound_mode: config.outbound_mode,
            inbound_mode: config.inbound_mode,
            live_model: config.live_model.clone(),
            gemini_live_models: config.gemini_live_models.clone(),
            openai_live_models: config.openai_live_models.clone(),
            soniox_live_models: config.soniox_live_models.clone(),
            summary_model: config.summary_model.clone(),
            summary_provider: config.summary_provider,
            echo_target_language: config.echo_target_language,
            vad_silence_duration_ms: config.vad_silence_duration_ms,
            vad_start_sensitivity: config.vad_start_sensitivity,
            vad_end_sensitivity: config.vad_end_sensitivity,
            keep_direct_audio: config.keep_direct_audio,
            inbound_original_under_translation: config.inbound_original_under_translation,
            inbound_original_ducked_gain: config.inbound_original_ducked_gain,
            close_to_tray: config.close_to_tray,
            theme_preference: config.theme_preference,
            proactive_session_refresh: config.proactive_session_refresh,
            record_meeting_audio: config.record_meeting_audio,
            meeting_audio_save_folder: config.meeting_audio_save_folder.clone(),
            transcript_layout: config.transcript_layout,
            overlay: config.overlay.clone(),
            outbound_voice_output: config.outbound_voice_output,
            inbound_voice_output: config.inbound_voice_output,
            soniox_api_key_configured: config.is_soniox_api_key_configured(),
            soniox: config.soniox.clone(),
            summary_fallback_available: config.summary_fallback_available(),
            elevenlabs_api_key_configured: config.is_elevenlabs_api_key_configured(),
            elevenlabs: ElevenLabsPublicSettings::from(&config.elevenlabs),
            artifacts_enabled: config.artifacts_enabled,
            answer_language: config.answer_language.clone(),
            meeting_context: config.meeting_context.clone(),
            custom_llm_profiles: config
                .custom_llm_profiles
                .iter()
                .map(|profile| crate::config::CustomLlmProfileView {
                    api_key_configured: config
                        .custom_llm_api_keys
                        .get(&profile.id)
                        .is_some_and(|k| !k.trim().is_empty()),
                    profile: profile.clone(),
                })
                .collect(),
            summary_custom_profile_id: config.summary_custom_profile_id.clone(),
        }
    }
}

impl ConfigView {
    pub fn to_app_config_partial(&self, existing: &AppConfig) -> AppConfig {
        AppConfig {
            ai_provider: self.ai_provider,
            gemini_api_key: existing.gemini_api_key.clone(),
            openai_api_key: existing.openai_api_key.clone(),
            soniox_api_key: existing.soniox_api_key.clone(),
            my_language: self.my_language.clone(),
            meeting_language: self.meeting_language.clone(),
            session_mode: self.session_mode,
            interpreter_my_language: self.interpreter_my_language.clone(),
            interpreter_meeting_language: self.interpreter_meeting_language.clone(),
            notes_language: self.notes_language.clone(),
            interpreter_outbound_mode: self.interpreter_outbound_mode,
            interpreter_inbound_mode: self.interpreter_inbound_mode,
            interpreter_outbound_voice_output: self.interpreter_outbound_voice_output,
            interpreter_inbound_voice_output: self.interpreter_inbound_voice_output,
            user_mic: self.user_mic.clone(),
            teams_mic_feed: self.teams_mic_feed.clone(),
            meeting_capture: self.meeting_capture.clone(),
            local_playback: self.local_playback.clone(),
            outbound_mode: self.outbound_mode,
            inbound_mode: self.inbound_mode,
            live_model: self.live_model.clone(),
            gemini_live_models: self.gemini_live_models.clone(),
            openai_live_models: self.openai_live_models.clone(),
            soniox_live_models: self.soniox_live_models.clone(),
            summary_model: self.summary_model.clone(),
            summary_provider: self.summary_provider,
            echo_target_language: self.echo_target_language,
            vad_silence_duration_ms: self.vad_silence_duration_ms,
            vad_start_sensitivity: self.vad_start_sensitivity,
            vad_end_sensitivity: self.vad_end_sensitivity,
            keep_direct_audio: self.keep_direct_audio,
            inbound_original_under_translation: self.inbound_original_under_translation,
            inbound_original_ducked_gain: self.inbound_original_ducked_gain,
            close_to_tray: self.close_to_tray,
            theme_preference: self.theme_preference,
            proactive_session_refresh: self.proactive_session_refresh,
            record_meeting_audio: self.record_meeting_audio,
            meeting_audio_save_folder: self.meeting_audio_save_folder.clone(),
            transcript_layout: self.transcript_layout,
            overlay: self.overlay.clone(),
            outbound_voice_output: self.outbound_voice_output,
            inbound_voice_output: self.inbound_voice_output,
            soniox: self.soniox.clone(),
            elevenlabs: self.elevenlabs.merge_into(&existing.elevenlabs),
            artifacts_enabled: self.artifacts_enabled,
            answer_language: self.answer_language.clone(),
            meeting_context: self.meeting_context.clone(),
            // Profiles are managed via dedicated CRUD commands — keep existing.
            custom_llm_profiles: existing.custom_llm_profiles.clone(),
            summary_custom_profile_id: existing.summary_custom_profile_id.clone(),
            custom_llm_api_keys: existing.custom_llm_api_keys.clone(),
        }
    }
}
