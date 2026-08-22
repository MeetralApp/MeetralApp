use anyhow::{Context, Result};
use serde::Deserialize;

use crate::ai::AiProvider;
use crate::config::{
    DeviceRef, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode, SessionMode,
    TranscriptLayout, VadSensitivity,
};

use super::stored::{
    default_elevenlabs_auto_mode, default_elevenlabs_chunk_schedule_preset,
    default_elevenlabs_crossfade_ms, default_elevenlabs_playback_crossfade,
    default_elevenlabs_similarity_boost, default_elevenlabs_speed, default_elevenlabs_stability,
    default_elevenlabs_tts_language_auto, default_elevenlabs_tts_model,
    default_elevenlabs_tts_synthesis_mode, default_elevenlabs_use_speaker_boost,
    default_live_model, default_meeting_language, default_my_language,
    default_soniox_endpoint_latency_level, default_soniox_endpoint_sensitivity,
    default_soniox_max_endpoint_delay_ms, default_soniox_tts_model, default_soniox_tts_speed,
    default_soniox_tts_voice, default_summary_model, default_summary_provider,
    stored_default_unified_outbound_topology, StoredConfig,
};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacyStoredConfig {
    #[serde(default)]
    encrypted_gemini_api_key: Option<String>,
    #[serde(default)]
    gemini_api_key: Option<String>,
    #[serde(default)]
    my_language: Option<String>,
    #[serde(default)]
    meeting_language: Option<String>,
    #[serde(default)]
    mic_device: Option<String>,
    #[serde(default)]
    outbound_playback_device: Option<String>,
    #[serde(default)]
    inbound_loopback_device: Option<String>,
    #[serde(default)]
    headphones_device: Option<String>,
    #[serde(default)]
    outbound_mode: Option<PipelineOutputMode>,
    #[serde(default)]
    inbound_mode: Option<PipelineOutputMode>,
    #[serde(default)]
    gemini_model: Option<String>,
    #[serde(default)]
    echo_target_language: Option<bool>,
    #[serde(default)]
    vad_silence_duration_ms: Option<u32>,
    #[serde(default)]
    vad_start_sensitivity: Option<VadSensitivity>,
    #[serde(default)]
    vad_end_sensitivity: Option<VadSensitivity>,
}

fn legacy_name(value: Option<String>) -> DeviceRef {
    DeviceRef {
        id: String::new(),
        name: value.unwrap_or_default(),
    }
}

impl LegacyStoredConfig {
    fn into_stored(self) -> StoredConfig {
        StoredConfig {
            ai_provider: AiProvider::Gemini,
            encrypted_gemini_api_key: self.encrypted_gemini_api_key,
            gemini_api_key: self.gemini_api_key,
            encrypted_openai_api_key: None,
            openai_api_key: None,
            encrypted_soniox_api_key: None,
            soniox_api_key: None,
            my_language: self.my_language.unwrap_or_else(default_my_language),
            meeting_language: self
                .meeting_language
                .unwrap_or_else(default_meeting_language),
            user_mic: legacy_name(self.mic_device),
            teams_mic_feed: legacy_name(self.outbound_playback_device),
            meeting_capture: legacy_name(self.inbound_loopback_device),
            local_playback: legacy_name(self.headphones_device),
            outbound_mode: self.outbound_mode.unwrap_or_default(),
            inbound_mode: self.inbound_mode.unwrap_or_default(),
            session_mode: SessionMode::Interpreter,
            interpreter_my_language: String::new(),
            interpreter_meeting_language: String::new(),
            notes_language: String::new(),
            interpreter_outbound_mode: None,
            interpreter_inbound_mode: None,
            interpreter_outbound_voice_output: None,
            interpreter_inbound_voice_output: None,
            live_model: self.gemini_model.unwrap_or_else(default_live_model),
            gemini_live_models: Vec::new(),
            openai_live_models: Vec::new(),
            soniox_live_models: Vec::new(),
            summary_model: default_summary_model(),
            summary_provider: default_summary_provider(),
            echo_target_language: self.echo_target_language.unwrap_or(true),
            vad_silence_duration_ms: self.vad_silence_duration_ms.unwrap_or(800),
            vad_start_sensitivity: self.vad_start_sensitivity.unwrap_or_default(),
            vad_end_sensitivity: self.vad_end_sensitivity.unwrap_or_default(),
            keep_direct_audio: true,
            inbound_original_under_translation: true,
            inbound_original_ducked_gain: 0.18,
            close_to_tray: true,
            theme_preference: crate::config::ThemePreference::Dark,
            proactive_session_refresh: false,
            transcript_layout: TranscriptLayout::SideBySide,
            overlay: crate::config::OverlaySettings::default(),
            outbound_voice_output: OutboundVoiceOutput::ProviderNative,
            inbound_voice_output: InboundVoiceOutput::ProviderNative,
            soniox_context_domain: String::new(),
            soniox_context_topic: String::new(),
            soniox_general: Vec::new(),
            soniox_context_text: String::new(),
            soniox_glossary_terms: Vec::new(),
            soniox_translation_terms: Vec::new(),
            soniox_always_on: crate::providers::shared::live::SonioxContextPayload::default(),
            soniox_context_profiles: Vec::new(),
            soniox_active_context_profile_id: None,
            soniox_tts_voice: default_soniox_tts_voice(),
            soniox_tts_outbound_voice: String::new(),
            soniox_tts_model: default_soniox_tts_model(),
            soniox_tts_voices: Vec::new(),
            soniox_tts_models: Vec::new(),
            soniox_tts_inbound_speed: default_soniox_tts_speed(),
            soniox_tts_outbound_speed: default_soniox_tts_speed(),
            soniox_endpoint_latency_adjustment_level: default_soniox_endpoint_latency_level(),
            soniox_endpoint_sensitivity: default_soniox_endpoint_sensitivity(),
            soniox_max_endpoint_delay_ms: default_soniox_max_endpoint_delay_ms(),
            encrypted_elevenlabs_api_key: None,
            elevenlabs_api_key: None,
            elevenlabs_voice_id: String::new(),
            elevenlabs_voices: Vec::new(),
            elevenlabs_models: Vec::new(),
            elevenlabs_tts_model: default_elevenlabs_tts_model(),
            elevenlabs_stability: default_elevenlabs_stability(),
            elevenlabs_similarity_boost: default_elevenlabs_similarity_boost(),
            elevenlabs_speed: default_elevenlabs_speed(),
            elevenlabs_use_speaker_boost: default_elevenlabs_use_speaker_boost(),
            elevenlabs_chunk_schedule_preset: default_elevenlabs_chunk_schedule_preset(),
            elevenlabs_tts_language_auto: default_elevenlabs_tts_language_auto(),
            elevenlabs_tts_language_code: String::new(),
            elevenlabs_tts_synthesis_mode: default_elevenlabs_tts_synthesis_mode(),
            elevenlabs_auto_mode: default_elevenlabs_auto_mode(),
            unified_outbound_topology: stored_default_unified_outbound_topology(),
            elevenlabs_playback_crossfade: default_elevenlabs_playback_crossfade(),
            elevenlabs_crossfade_ms: default_elevenlabs_crossfade_ms(),
            elevenlabs_inbound_voice_id: String::new(),
            elevenlabs_inbound_tts_model: default_elevenlabs_tts_model(),
            elevenlabs_inbound_stability: default_elevenlabs_stability(),
            elevenlabs_inbound_similarity_boost: default_elevenlabs_similarity_boost(),
            elevenlabs_inbound_tts_synthesis_mode: default_elevenlabs_tts_synthesis_mode(),
            record_meeting_audio: false,
            meeting_audio_save_folder: String::new(),
            artifacts_enabled: true,
            answer_language: String::new(),
            meeting_context: crate::config::MeetingContextPayload::default(),
            custom_llm_profiles: Vec::new(),
            summary_custom_profile_id: None,
            encrypted_custom_llm_keys: std::collections::HashMap::new(),
        }
    }
}

pub(crate) fn parse_stored(bytes: &[u8]) -> Result<StoredConfig> {
    if let Ok(stored) = serde_json::from_slice::<StoredConfig>(bytes) {
        return Ok(stored);
    }

    let legacy: LegacyStoredConfig =
        serde_json::from_slice(bytes).context("parse legacy config.json")?;
    Ok(legacy.into_stored())
}
