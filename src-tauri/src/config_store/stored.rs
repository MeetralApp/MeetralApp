use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

use crate::ai::AiProvider;
use crate::config::{
    AppConfig, DeviceRef, InboundVoiceOutput, OutboundVoiceOutput, OverlaySettings,
    PipelineOutputMode, SessionMode, ThemePreference, TranscriptLayout, VadSensitivity,
};
use crate::secret;

use super::soniox_migrate::migrate_soniox_always_on;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredConfig {
    #[serde(default)]
    pub(crate) ai_provider: AiProvider,
    #[serde(default)]
    pub(crate) encrypted_gemini_api_key: Option<String>,
    #[serde(default)]
    pub(crate) gemini_api_key: Option<String>,
    #[serde(default)]
    pub(crate) encrypted_openai_api_key: Option<String>,
    #[serde(default)]
    pub(crate) openai_api_key: Option<String>,
    #[serde(default)]
    pub(crate) encrypted_soniox_api_key: Option<String>,
    #[serde(default)]
    pub(crate) soniox_api_key: Option<String>,
    #[serde(default = "default_my_language")]
    pub(crate) my_language: String,
    #[serde(default = "default_meeting_language")]
    pub(crate) meeting_language: String,
    #[serde(default)]
    pub(crate) session_mode: SessionMode,
    #[serde(default)]
    pub(crate) interpreter_my_language: String,
    #[serde(default)]
    pub(crate) interpreter_meeting_language: String,
    #[serde(default)]
    pub(crate) notes_language: String,
    #[serde(default)]
    pub(crate) interpreter_outbound_mode: Option<PipelineOutputMode>,
    #[serde(default)]
    pub(crate) interpreter_inbound_mode: Option<PipelineOutputMode>,
    #[serde(default)]
    pub(crate) interpreter_outbound_voice_output: Option<OutboundVoiceOutput>,
    #[serde(default)]
    pub(crate) interpreter_inbound_voice_output: Option<InboundVoiceOutput>,
    #[serde(default)]
    pub(crate) user_mic: DeviceRef,
    #[serde(default)]
    pub(crate) teams_mic_feed: DeviceRef,
    #[serde(default)]
    pub(crate) meeting_capture: DeviceRef,
    #[serde(default)]
    pub(crate) local_playback: DeviceRef,
    #[serde(default)]
    pub(crate) outbound_mode: PipelineOutputMode,
    #[serde(default)]
    pub(crate) inbound_mode: PipelineOutputMode,
    #[serde(default = "default_live_model", alias = "geminiModel")]
    pub(crate) live_model: String,
    #[serde(default)]
    pub(crate) gemini_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default)]
    pub(crate) openai_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default)]
    pub(crate) soniox_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default = "default_summary_model", alias = "geminiSummaryModel")]
    pub(crate) summary_model: String,
    #[serde(default = "default_summary_provider")]
    pub(crate) summary_provider: AiProvider,
    #[serde(default = "default_true")]
    pub(crate) echo_target_language: bool,
    #[serde(default = "default_vad_silence")]
    pub(crate) vad_silence_duration_ms: u32,
    #[serde(default)]
    pub(crate) vad_start_sensitivity: VadSensitivity,
    #[serde(default)]
    pub(crate) vad_end_sensitivity: VadSensitivity,
    #[serde(default = "default_true")]
    pub(crate) keep_direct_audio: bool,
    #[serde(default = "default_true")]
    pub(crate) inbound_original_under_translation: bool,
    #[serde(default = "default_inbound_original_ducked_gain")]
    pub(crate) inbound_original_ducked_gain: f32,
    #[serde(default = "default_true")]
    pub(crate) close_to_tray: bool,
    #[serde(default)]
    pub(crate) theme_preference: ThemePreference,
    #[serde(default)]
    pub(crate) proactive_session_refresh: bool,
    #[serde(default)]
    pub(crate) record_meeting_audio: bool,
    #[serde(default)]
    pub(crate) meeting_audio_save_folder: String,
    #[serde(default)]
    pub(crate) transcript_layout: TranscriptLayout,
    #[serde(default)]
    pub(crate) overlay: OverlaySettings,
    #[serde(default)]
    pub(crate) outbound_voice_output: OutboundVoiceOutput,
    #[serde(default)]
    pub(crate) inbound_voice_output: InboundVoiceOutput,
    /// Legacy fields — migrated into Always-on on load.
    #[serde(default)]
    pub(crate) soniox_context_domain: String,
    #[serde(default)]
    pub(crate) soniox_context_topic: String,
    #[serde(default)]
    pub(crate) soniox_general: Vec<crate::providers::shared::live::SonioxGeneralPair>,
    #[serde(default)]
    pub(crate) soniox_context_text: String,
    #[serde(default)]
    pub(crate) soniox_glossary_terms: Vec<String>,
    #[serde(default)]
    pub(crate) soniox_translation_terms: Vec<crate::providers::shared::live::SonioxTranslationTerm>,
    #[serde(default)]
    pub(crate) soniox_always_on: crate::providers::shared::live::SonioxContextPayload,
    #[serde(default)]
    pub(crate) soniox_context_profiles: Vec<crate::providers::shared::live::SonioxContextProfile>,
    #[serde(default)]
    pub(crate) soniox_active_context_profile_id: Option<String>,
    #[serde(default = "default_soniox_tts_voice")]
    pub(crate) soniox_tts_voice: String,
    #[serde(default)]
    pub(crate) soniox_tts_outbound_voice: String,
    #[serde(default = "default_soniox_tts_model")]
    pub(crate) soniox_tts_model: String,
    #[serde(default)]
    pub(crate) soniox_tts_voices: Vec<crate::voice::SonioxVoiceOption>,
    #[serde(default)]
    pub(crate) soniox_tts_models: Vec<crate::voice::SonioxTtsModelOption>,
    #[serde(default = "default_soniox_tts_speed")]
    pub(crate) soniox_tts_inbound_speed: f32,
    #[serde(default = "default_soniox_tts_speed")]
    pub(crate) soniox_tts_outbound_speed: f32,
    #[serde(default = "default_soniox_endpoint_latency_level")]
    pub(crate) soniox_endpoint_latency_adjustment_level: u8,
    #[serde(default = "default_soniox_endpoint_sensitivity")]
    pub(crate) soniox_endpoint_sensitivity: f64,
    #[serde(default = "default_soniox_max_endpoint_delay_ms")]
    pub(crate) soniox_max_endpoint_delay_ms: u32,
    #[serde(default)]
    pub(crate) encrypted_elevenlabs_api_key: Option<String>,
    #[serde(default)]
    pub(crate) elevenlabs_api_key: Option<String>,
    #[serde(default)]
    pub(crate) elevenlabs_voice_id: String,
    #[serde(default)]
    pub(crate) elevenlabs_voices: Vec<crate::voice::ElevenLabsVoiceOption>,
    #[serde(default)]
    pub(crate) elevenlabs_models: Vec<crate::voice::ElevenLabsModelOption>,
    #[serde(default = "default_elevenlabs_tts_model")]
    pub(crate) elevenlabs_tts_model: String,
    #[serde(default = "default_elevenlabs_stability")]
    pub(crate) elevenlabs_stability: f32,
    #[serde(default = "default_elevenlabs_similarity_boost")]
    pub(crate) elevenlabs_similarity_boost: f32,
    #[serde(default = "default_elevenlabs_speed")]
    pub(crate) elevenlabs_speed: f32,
    #[serde(default = "default_elevenlabs_use_speaker_boost")]
    pub(crate) elevenlabs_use_speaker_boost: bool,
    #[serde(default = "default_elevenlabs_chunk_schedule_preset")]
    pub(crate) elevenlabs_chunk_schedule_preset:
        crate::voice::config::ElevenLabsChunkSchedulePreset,
    #[serde(default = "default_elevenlabs_tts_language_auto")]
    pub(crate) elevenlabs_tts_language_auto: bool,
    #[serde(default)]
    pub(crate) elevenlabs_tts_language_code: String,
    #[serde(default = "default_elevenlabs_tts_synthesis_mode")]
    pub(crate) elevenlabs_tts_synthesis_mode: crate::voice::config::TtsSynthesisMode,
    #[serde(default = "default_elevenlabs_auto_mode")]
    pub(crate) elevenlabs_auto_mode: bool,
    #[serde(default = "stored_default_unified_outbound_topology")]
    pub(crate) unified_outbound_topology: bool,
    #[serde(default = "default_elevenlabs_playback_crossfade")]
    pub(crate) elevenlabs_playback_crossfade: bool,
    #[serde(default = "default_true")]
    pub(crate) artifacts_enabled: bool,
    #[serde(default)]
    pub(crate) answer_language: String,
    /// App-level meeting context for summary prompts — additive.
    #[serde(default)]
    pub(crate) meeting_context: crate::config::MeetingContextPayload,
    /// Custom OpenAI-compatible LLM profiles — additive, default [].
    #[serde(default)]
    pub(crate) custom_llm_profiles: Vec<crate::config::CustomLlmProfile>,
    #[serde(default)]
    pub(crate) summary_custom_profile_id: Option<String>,
    /// profile_id → base64(DPAPI/keychain ciphertext). Plaintext keys never
    /// persist; entries without a matching profile are GC'd on load.
    #[serde(default)]
    pub(crate) encrypted_custom_llm_keys: std::collections::HashMap<String, String>,
    #[serde(default = "default_elevenlabs_crossfade_ms")]
    pub(crate) elevenlabs_crossfade_ms: u32,
    #[serde(default)]
    pub(crate) elevenlabs_inbound_voice_id: String,
    #[serde(default = "default_elevenlabs_tts_model")]
    pub(crate) elevenlabs_inbound_tts_model: String,
    #[serde(default = "default_elevenlabs_stability")]
    pub(crate) elevenlabs_inbound_stability: f32,
    #[serde(default = "default_elevenlabs_similarity_boost")]
    pub(crate) elevenlabs_inbound_similarity_boost: f32,
    #[serde(default = "default_elevenlabs_tts_synthesis_mode")]
    pub(crate) elevenlabs_inbound_tts_synthesis_mode: crate::voice::config::TtsSynthesisMode,
}

pub(crate) fn stored_default_unified_outbound_topology() -> bool {
    true
}

pub(crate) fn default_my_language() -> String {
    "vi".to_string()
}

pub(crate) fn default_meeting_language() -> String {
    "en".to_string()
}

pub(crate) fn default_live_model() -> String {
    AppConfig::default().live_model
}

pub(crate) fn default_summary_model() -> String {
    AppConfig::default().summary_model
}

pub(crate) fn default_summary_provider() -> AiProvider {
    AiProvider::Gemini
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_inbound_original_ducked_gain() -> f32 {
    0.18
}

pub(crate) fn default_vad_silence() -> u32 {
    800
}

pub(crate) fn default_elevenlabs_tts_model() -> String {
    crate::voice::default_elevenlabs_tts_model()
}

pub(crate) fn default_elevenlabs_stability() -> f32 {
    crate::voice::default_elevenlabs_stability()
}

pub(crate) fn default_elevenlabs_similarity_boost() -> f32 {
    crate::voice::default_elevenlabs_similarity_boost()
}

pub(crate) fn default_elevenlabs_speed() -> f32 {
    crate::voice::default_elevenlabs_speed()
}

pub(crate) fn default_elevenlabs_use_speaker_boost() -> bool {
    crate::voice::default_elevenlabs_use_speaker_boost()
}

pub(crate) fn default_elevenlabs_chunk_schedule_preset(
) -> crate::voice::config::ElevenLabsChunkSchedulePreset {
    crate::voice::default_elevenlabs_chunk_schedule_preset()
}

pub(crate) fn default_elevenlabs_tts_synthesis_mode() -> crate::voice::config::TtsSynthesisMode {
    crate::voice::default_elevenlabs_tts_synthesis_mode()
}

pub(crate) fn default_elevenlabs_auto_mode() -> bool {
    crate::voice::config::default_elevenlabs_auto_mode()
}

pub(crate) fn default_elevenlabs_playback_crossfade() -> bool {
    crate::voice::config::default_elevenlabs_playback_crossfade()
}

pub(crate) fn default_elevenlabs_crossfade_ms() -> u32 {
    crate::voice::config::default_elevenlabs_crossfade_ms()
}

pub(crate) fn default_soniox_tts_voice() -> String {
    crate::providers::soniox::tts::config::default_soniox_tts_voice()
}

pub(crate) fn default_soniox_tts_model() -> String {
    crate::providers::soniox::tts::config::default_soniox_tts_model()
}

pub(crate) fn default_soniox_tts_speed() -> f32 {
    crate::providers::soniox::tts::config::default_soniox_tts_speed()
}

pub(crate) fn default_soniox_endpoint_latency_level() -> u8 {
    crate::providers::soniox::config::ENDPOINT_LATENCY_ADJUSTMENT_LEVEL
}

pub(crate) fn default_soniox_endpoint_sensitivity() -> f64 {
    crate::providers::soniox::config::ENDPOINT_SENSITIVITY
}

pub(crate) fn default_soniox_max_endpoint_delay_ms() -> u32 {
    crate::providers::soniox::config::MAX_ENDPOINT_DELAY_MS
}

pub(crate) fn default_elevenlabs_tts_language_auto() -> bool {
    crate::voice::default_elevenlabs_tts_language_auto()
}

fn read_elevenlabs_api_key(stored: &StoredConfig) -> Result<String> {
    if let Some(encoded) = stored.encrypted_elevenlabs_api_key.as_ref() {
        if encoded.is_empty() {
            return Ok(String::new());
        }
        let bytes = STANDARD
            .decode(encoded)
            .context("decode encrypted ElevenLabs API key")?;
        return secret::decrypt_for_account(crate::voice::ELEVENLABS_KEYCHAIN_ACCOUNT, &bytes);
    }
    Ok(stored.elevenlabs_api_key.clone().unwrap_or_default())
}

fn write_elevenlabs_api_key(key: &str) -> Result<Option<String>> {
    if key.trim().is_empty() {
        return Ok(None);
    }
    let encrypted = secret::encrypt_for_account(crate::voice::ELEVENLABS_KEYCHAIN_ACCOUNT, key)?;
    Ok(Some(STANDARD.encode(encrypted)))
}

fn read_provider_api_key(stored: &StoredConfig, provider: AiProvider) -> Result<String> {
    let (encrypted, plaintext) = match provider {
        AiProvider::Gemini => (
            stored.encrypted_gemini_api_key.as_ref(),
            stored.gemini_api_key.as_ref(),
        ),
        AiProvider::OpenAi => (
            stored.encrypted_openai_api_key.as_ref(),
            stored.openai_api_key.as_ref(),
        ),
        AiProvider::Soniox => (
            stored.encrypted_soniox_api_key.as_ref(),
            stored.soniox_api_key.as_ref(),
        ),
    };

    if let Some(encoded) = encrypted {
        if encoded.is_empty() {
            return Ok(String::new());
        }
        let bytes = STANDARD
            .decode(encoded)
            .with_context(|| format!("decode encrypted {} API key", provider.label()))?;
        return secret::decrypt_for_account(provider.keychain_account(), &bytes);
    }

    Ok(plaintext.cloned().unwrap_or_default())
}

fn write_provider_api_key(provider: AiProvider, key: &str) -> Result<Option<String>> {
    if key.trim().is_empty() {
        return Ok(None);
    }
    let encrypted = secret::encrypt_for_account(provider.keychain_account(), key)?;
    Ok(Some(STANDARD.encode(encrypted)))
}

/// Decrypt custom-profile keys. Orphan entries (profile deleted without
/// a save) are GC'd: dropped from the map — the next save purges them from
/// disk — and deleted from the macOS keychain best-effort.
fn read_custom_llm_keys(
    stored: &StoredConfig,
) -> Result<std::collections::HashMap<String, String>> {
    let mut keys = std::collections::HashMap::new();
    for (id, encoded) in &stored.encrypted_custom_llm_keys {
        let profile_exists = stored.custom_llm_profiles.iter().any(|p| &p.id == id);
        if !profile_exists {
            tracing::warn!(profile_id = %id, "GC orphan custom LLM key (profile gone)");
            let account = format!("custom_llm:{id}");
            if let Err(error) = secret::delete_for_account(&account) {
                tracing::warn!(profile_id = %id, error = %error, "orphan keychain delete failed");
            }
            continue;
        }
        if encoded.is_empty() {
            continue;
        }
        let bytes = STANDARD
            .decode(encoded)
            .with_context(|| format!("decode encrypted custom LLM key ({id})"))?;
        let account = format!("custom_llm:{id}");
        let key = secret::decrypt_for_account(&account, &bytes)?;
        if !key.trim().is_empty() {
            keys.insert(id.clone(), key);
        }
    }
    Ok(keys)
}

fn write_custom_llm_keys(config: &AppConfig) -> Result<std::collections::HashMap<String, String>> {
    let mut out = std::collections::HashMap::new();
    for profile in &config.custom_llm_profiles {
        let Some(key) = config.custom_llm_api_keys.get(&profile.id) else {
            continue;
        };
        if key.trim().is_empty() {
            continue;
        }
        let encrypted = secret::encrypt_for_account(&profile.keychain_account(), key)?;
        out.insert(profile.id.clone(), STANDARD.encode(encrypted));
    }
    Ok(out)
}

impl StoredConfig {
    pub(crate) fn into_app_config(self) -> Result<AppConfig> {
        let elevenlabs_api_key = read_elevenlabs_api_key(&self)?;
        // Computed before the struct literal: `self` fields move during it,
        // so a later `&self` borrow would not compile.
        let custom_llm_api_keys = read_custom_llm_keys(&self)?;
        let mut config = AppConfig {
            ai_provider: self.ai_provider,
            gemini_api_key: read_provider_api_key(&self, AiProvider::Gemini)?,
            openai_api_key: read_provider_api_key(&self, AiProvider::OpenAi)?,
            soniox_api_key: read_provider_api_key(&self, AiProvider::Soniox)?,
            my_language: self.my_language,
            meeting_language: self.meeting_language,
            session_mode: self.session_mode,
            interpreter_my_language: self.interpreter_my_language,
            interpreter_meeting_language: self.interpreter_meeting_language,
            notes_language: self.notes_language,
            interpreter_outbound_mode: self.interpreter_outbound_mode,
            interpreter_inbound_mode: self.interpreter_inbound_mode,
            interpreter_outbound_voice_output: self.interpreter_outbound_voice_output,
            interpreter_inbound_voice_output: self.interpreter_inbound_voice_output,
            user_mic: self.user_mic,
            teams_mic_feed: self.teams_mic_feed,
            meeting_capture: self.meeting_capture,
            local_playback: self.local_playback,
            outbound_mode: self.outbound_mode,
            inbound_mode: self.inbound_mode,
            live_model: self.live_model,
            gemini_live_models: self.gemini_live_models,
            openai_live_models: self.openai_live_models,
            soniox_live_models: self.soniox_live_models,
            summary_model: self.summary_model,
            summary_provider: crate::config::app_config_validate::normalize_summary_provider(
                self.summary_provider,
            ),
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
            meeting_audio_save_folder: self.meeting_audio_save_folder,
            transcript_layout: self.transcript_layout,
            overlay: self.overlay,
            outbound_voice_output: self.outbound_voice_output,
            inbound_voice_output: self.inbound_voice_output,
            soniox: crate::config::SonioxSettings {
                soniox_always_on: migrate_soniox_always_on(
                    self.soniox_always_on,
                    &self.soniox_context_profiles,
                    &self.soniox_active_context_profile_id,
                    self.soniox_general,
                    self.soniox_context_text,
                    self.soniox_glossary_terms,
                    self.soniox_translation_terms,
                    &self.soniox_context_domain,
                    &self.soniox_context_topic,
                ),
                soniox_context_profiles: self.soniox_context_profiles,
                soniox_active_context_profile_id: self.soniox_active_context_profile_id,
                soniox_tts_voice: self.soniox_tts_voice,
                soniox_tts_outbound_voice: self.soniox_tts_outbound_voice,
                soniox_tts_model: self.soniox_tts_model,
                soniox_tts_voices: self.soniox_tts_voices,
                soniox_tts_models: self.soniox_tts_models,
                soniox_tts_inbound_speed: self.soniox_tts_inbound_speed,
                soniox_tts_outbound_speed: self.soniox_tts_outbound_speed,
                soniox_endpoint_latency_adjustment_level: self
                    .soniox_endpoint_latency_adjustment_level,
                soniox_endpoint_sensitivity: self.soniox_endpoint_sensitivity,
                soniox_max_endpoint_delay_ms: self.soniox_max_endpoint_delay_ms,
            },
            elevenlabs: crate::config::ElevenLabsSettings {
                elevenlabs_api_key,
                elevenlabs_voice_id: self.elevenlabs_voice_id.clone(),
                elevenlabs_voices: self.elevenlabs_voices.clone(),
                elevenlabs_models: self.elevenlabs_models.clone(),
                elevenlabs_tts_model: self.elevenlabs_tts_model.clone(),
                elevenlabs_stability: self.elevenlabs_stability,
                elevenlabs_similarity_boost: self.elevenlabs_similarity_boost,
                elevenlabs_speed: self.elevenlabs_speed,
                elevenlabs_use_speaker_boost: self.elevenlabs_use_speaker_boost,
                elevenlabs_chunk_schedule_preset: self.elevenlabs_chunk_schedule_preset,
                elevenlabs_tts_language_auto: self.elevenlabs_tts_language_auto,
                elevenlabs_tts_language_code: self.elevenlabs_tts_language_code.clone(),
                elevenlabs_tts_synthesis_mode: self.elevenlabs_tts_synthesis_mode,
                elevenlabs_auto_mode: self.elevenlabs_auto_mode,
                unified_outbound_topology: self.unified_outbound_topology,
                elevenlabs_playback_crossfade: self.elevenlabs_playback_crossfade,
                elevenlabs_crossfade_ms: self.elevenlabs_crossfade_ms,
                elevenlabs_inbound_voice_id: self.elevenlabs_inbound_voice_id.clone(),
                elevenlabs_inbound_tts_model: self.elevenlabs_inbound_tts_model.clone(),
                elevenlabs_inbound_stability: self.elevenlabs_inbound_stability,
                elevenlabs_inbound_similarity_boost: self.elevenlabs_inbound_similarity_boost,
                elevenlabs_inbound_tts_synthesis_mode: self.elevenlabs_inbound_tts_synthesis_mode,
            },
            artifacts_enabled: self.artifacts_enabled,
            answer_language: self.answer_language,
            meeting_context: self.meeting_context,
            custom_llm_profiles: self.custom_llm_profiles.clone(),
            summary_custom_profile_id: self.summary_custom_profile_id.clone(),
            custom_llm_api_keys,
        };
        config.normalize();
        Ok(config)
    }

    pub(crate) fn from_app_config(config: &AppConfig) -> Result<Self> {
        Ok(Self {
            ai_provider: config.ai_provider,
            encrypted_gemini_api_key: write_provider_api_key(
                AiProvider::Gemini,
                &config.gemini_api_key,
            )?,
            gemini_api_key: None,
            encrypted_openai_api_key: write_provider_api_key(
                AiProvider::OpenAi,
                &config.openai_api_key,
            )?,
            openai_api_key: None,
            encrypted_soniox_api_key: write_provider_api_key(
                AiProvider::Soniox,
                &config.soniox_api_key,
            )?,
            soniox_api_key: None,
            encrypted_elevenlabs_api_key: write_elevenlabs_api_key(
                &config.elevenlabs.elevenlabs_api_key,
            )?,
            elevenlabs_api_key: None,
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
            soniox_context_domain: String::new(),
            soniox_context_topic: String::new(),
            soniox_general: Vec::new(),
            soniox_context_text: String::new(),
            soniox_glossary_terms: Vec::new(),
            soniox_translation_terms: Vec::new(),
            soniox_always_on: config.soniox.soniox_always_on.clone(),
            soniox_context_profiles: config.soniox.soniox_context_profiles.clone(),
            soniox_active_context_profile_id: config
                .soniox
                .soniox_active_context_profile_id
                .clone(),
            soniox_tts_voice: config.soniox.soniox_tts_voice.clone(),
            soniox_tts_outbound_voice: config.soniox.soniox_tts_outbound_voice.clone(),
            soniox_tts_model: config.soniox.soniox_tts_model.clone(),
            soniox_tts_voices: config.soniox.soniox_tts_voices.clone(),
            soniox_tts_models: config.soniox.soniox_tts_models.clone(),
            soniox_tts_inbound_speed: config.soniox.soniox_tts_inbound_speed,
            soniox_tts_outbound_speed: config.soniox.soniox_tts_outbound_speed,
            soniox_endpoint_latency_adjustment_level: config
                .soniox
                .soniox_endpoint_latency_adjustment_level,
            soniox_endpoint_sensitivity: config.soniox.soniox_endpoint_sensitivity,
            soniox_max_endpoint_delay_ms: config.soniox.soniox_max_endpoint_delay_ms,
            elevenlabs_voice_id: config.elevenlabs.elevenlabs_voice_id.clone(),
            elevenlabs_voices: config.elevenlabs.elevenlabs_voices.clone(),
            elevenlabs_models: config.elevenlabs.elevenlabs_models.clone(),
            elevenlabs_tts_model: config.elevenlabs.elevenlabs_tts_model.clone(),
            elevenlabs_stability: config.elevenlabs.elevenlabs_stability,
            elevenlabs_similarity_boost: config.elevenlabs.elevenlabs_similarity_boost,
            elevenlabs_speed: config.elevenlabs.elevenlabs_speed,
            elevenlabs_use_speaker_boost: config.elevenlabs.elevenlabs_use_speaker_boost,
            elevenlabs_chunk_schedule_preset: config.elevenlabs.elevenlabs_chunk_schedule_preset,
            elevenlabs_tts_language_auto: config.elevenlabs.elevenlabs_tts_language_auto,
            elevenlabs_tts_language_code: config.elevenlabs.elevenlabs_tts_language_code.clone(),
            elevenlabs_tts_synthesis_mode: config.elevenlabs.elevenlabs_tts_synthesis_mode,
            elevenlabs_auto_mode: config.elevenlabs.elevenlabs_auto_mode,
            unified_outbound_topology: config.elevenlabs.unified_outbound_topology,
            elevenlabs_playback_crossfade: config.elevenlabs.elevenlabs_playback_crossfade,
            elevenlabs_crossfade_ms: config.elevenlabs.elevenlabs_crossfade_ms,
            elevenlabs_inbound_voice_id: config.elevenlabs.elevenlabs_inbound_voice_id.clone(),
            elevenlabs_inbound_tts_model: config.elevenlabs.elevenlabs_inbound_tts_model.clone(),
            elevenlabs_inbound_stability: config.elevenlabs.elevenlabs_inbound_stability,
            elevenlabs_inbound_similarity_boost: config
                .elevenlabs
                .elevenlabs_inbound_similarity_boost,
            elevenlabs_inbound_tts_synthesis_mode: config
                .elevenlabs
                .elevenlabs_inbound_tts_synthesis_mode,
            artifacts_enabled: config.artifacts_enabled,
            answer_language: config.answer_language.clone(),
            meeting_context: config.meeting_context.clone(),
            custom_llm_profiles: config.custom_llm_profiles.clone(),
            summary_custom_profile_id: config.summary_custom_profile_id.clone(),
            encrypted_custom_llm_keys: write_custom_llm_keys(config)?,
        })
    }
}
