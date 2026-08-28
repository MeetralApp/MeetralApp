use serde::{Deserialize, Serialize};

use crate::ai::{
    default_live_model_for_provider, normalize_summary_model_for_provider, AiProvider,
};

use super::device::DeviceRef;
use super::elevenlabs_settings::ElevenLabsSettings;
use super::fishaudio_settings::FishAudioSettings;
use super::modes::{
    CustomVoiceVendor, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode, SessionMode,
    ThemePreference, TranscriptLayout, VadSensitivity,
};
use super::overlay_settings::OverlaySettings;
use super::soniox_settings::SonioxSettings;
use super::xai_settings::XaiSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub ai_provider: AiProvider,
    pub gemini_api_key: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub soniox_api_key: String,
    pub my_language: String,
    pub meeting_language: String,
    #[serde(default)]
    pub session_mode: SessionMode,
    /// Interpreter “You” language stash (projected into `my_language` when interpreter).
    #[serde(default)]
    pub interpreter_my_language: String,
    /// Interpreter “Meeting” language stash.
    #[serde(default)]
    pub interpreter_meeting_language: String,
    /// Notes single-language stash (projected into both active langs when notes).
    #[serde(default)]
    pub notes_language: String,
    /// Interpreter outbound pipeline mode stash.
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
    #[serde(default)]
    pub outbound_mode: PipelineOutputMode,
    #[serde(default)]
    pub inbound_mode: PipelineOutputMode,
    #[serde(default = "default_live_model_field", alias = "geminiModel")]
    pub live_model: String,
    /// Persisted live model catalogs (seeded static or Soniox API).
    #[serde(default)]
    pub gemini_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default)]
    pub openai_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default)]
    pub soniox_live_models: Vec<crate::ai::LiveModelOption>,
    #[serde(default = "default_summary_model_field", alias = "geminiSummaryModel")]
    pub summary_model: String,
    /// LLM used for meeting summaries (Gemini or OpenAI only; Soniox normalizes to Gemini).
    #[serde(default = "default_summary_provider_field")]
    pub summary_provider: AiProvider,
    #[serde(default = "default_true")]
    pub echo_target_language: bool,
    #[serde(default = "default_vad_silence")]
    pub vad_silence_duration_ms: u32,
    #[serde(default)]
    pub vad_start_sensitivity: VadSensitivity,
    #[serde(default)]
    pub vad_end_sensitivity: VadSensitivity,
    #[serde(default = "default_true")]
    pub keep_direct_audio: bool,
    /// Mix continuous meeting floor under Meeting→You translation (fixed gain).
    #[serde(default = "default_true")]
    pub inbound_original_under_translation: bool,
    #[serde(default = "default_inbound_original_ducked_gain")]
    pub inbound_original_ducked_gain: f32,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    /// End the live meeting after no new transcript segment for `auto_end_meeting_after_min`.
    #[serde(default)]
    pub auto_end_meeting: bool,
    #[serde(default = "default_auto_end_meeting_after_min")]
    pub auto_end_meeting_after_min: u32,
    #[serde(default)]
    pub theme_preference: ThemePreference,
    #[serde(default)]
    pub proactive_session_refresh: bool,
    #[serde(default)]
    pub record_meeting_audio: bool,
    /// Empty = use `{app_data}/recordings`. Absolute path when user picks a folder.
    #[serde(default)]
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
    pub outbound_custom_voice_vendor: CustomVoiceVendor,
    #[serde(default)]
    pub inbound_custom_voice_vendor: CustomVoiceVendor,
    #[serde(default, flatten)]
    pub soniox: SonioxSettings,
    #[serde(default, flatten)]
    pub elevenlabs: ElevenLabsSettings,
    #[serde(default, flatten)]
    pub fishaudio: FishAudioSettings,
    #[serde(default, flatten)]
    pub xai: XaiSettings,
    /// Meeting Intelligence: structured artifacts (decisions/action items/entities).
    #[serde(default = "default_true")]
    pub artifacts_enabled: bool,
    /// Meeting Intelligence: preferred AI output language for summaries.
    /// Empty = match the meeting's "You" language.
    #[serde(default)]
    pub answer_language: String,
    /// App-level meeting context (domain / terminology) injected into summary
    /// prompts. Shares the Soniox 4-section context model.
    #[serde(default)]
    pub meeting_context: super::meeting_context::MeetingContextPayload,
    /// Custom OpenAI-compatible LLM profiles — local servers as DATA.
    #[serde(default)]
    pub custom_llm_profiles: Vec<super::custom_llm::CustomLlmProfile>,
    /// When set, summaries use this custom profile instead of the built-in
    /// `summary_provider`.
    #[serde(default)]
    pub summary_custom_profile_id: Option<String>,
    /// In-memory plaintext keys for custom profiles (decrypted at load from
    /// `encrypted_custom_llm_keys`; never serialized).
    #[serde(skip)]
    pub custom_llm_api_keys: std::collections::HashMap<String, String>,
}

pub(crate) fn default_live_model_field() -> String {
    default_live_model_for_provider(AiProvider::Gemini)
}

pub(crate) fn default_summary_model_field() -> String {
    crate::ai::default_summary_model_for_provider(AiProvider::Gemini)
}

pub(crate) fn default_summary_provider_field() -> AiProvider {
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

/// Default idle duration when auto-end is on (Settings preset).
pub const DEFAULT_AUTO_END_MEETING_AFTER_MIN: u32 = 5;
pub const AUTO_END_MEETING_AFTER_MIN_PRESETS: [u32; 4] = [1, 5, 10, 15];

pub(crate) fn default_auto_end_meeting_after_min() -> u32 {
    DEFAULT_AUTO_END_MEETING_AFTER_MIN
}

/// Snap a stored/typed minute value onto the Settings presets.
pub(crate) fn normalize_auto_end_meeting_after_min(value: u32) -> u32 {
    AUTO_END_MEETING_AFTER_MIN_PRESETS
        .iter()
        .copied()
        .min_by_key(|preset| preset.abs_diff(value))
        .unwrap_or(DEFAULT_AUTO_END_MEETING_AFTER_MIN)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai_provider: AiProvider::Gemini,
            gemini_api_key: String::new(),
            openai_api_key: String::new(),
            soniox_api_key: String::new(),
            my_language: "vi".to_string(),
            meeting_language: "en".to_string(),
            session_mode: SessionMode::Interpreter,
            interpreter_my_language: "vi".to_string(),
            interpreter_meeting_language: "en".to_string(),
            notes_language: "vi".to_string(),
            interpreter_outbound_mode: Some(PipelineOutputMode::Translated),
            interpreter_inbound_mode: Some(PipelineOutputMode::Translated),
            interpreter_outbound_voice_output: Some(OutboundVoiceOutput::ProviderNative),
            interpreter_inbound_voice_output: Some(InboundVoiceOutput::ProviderNative),
            user_mic: DeviceRef::empty(),
            teams_mic_feed: DeviceRef::empty(),
            meeting_capture: DeviceRef::empty(),
            local_playback: DeviceRef::empty(),
            outbound_mode: PipelineOutputMode::Translated,
            inbound_mode: PipelineOutputMode::Translated,
            live_model: default_live_model_for_provider(AiProvider::Gemini),
            gemini_live_models: Vec::new(),
            openai_live_models: Vec::new(),
            soniox_live_models: Vec::new(),
            summary_model: crate::ai::default_summary_model_for_provider(AiProvider::Gemini),
            summary_provider: AiProvider::Gemini,
            echo_target_language: true,
            vad_silence_duration_ms: 800,
            vad_start_sensitivity: VadSensitivity::Low,
            vad_end_sensitivity: VadSensitivity::Low,
            keep_direct_audio: true,
            inbound_original_under_translation: true,
            inbound_original_ducked_gain: default_inbound_original_ducked_gain(),
            close_to_tray: true,
            auto_end_meeting: false,
            auto_end_meeting_after_min: DEFAULT_AUTO_END_MEETING_AFTER_MIN,
            theme_preference: ThemePreference::Dark,
            proactive_session_refresh: false,
            record_meeting_audio: false,
            meeting_audio_save_folder: String::new(),
            transcript_layout: TranscriptLayout::SideBySide,
            overlay: OverlaySettings::default(),
            outbound_voice_output: OutboundVoiceOutput::ProviderNative,
            inbound_voice_output: InboundVoiceOutput::ProviderNative,
            outbound_custom_voice_vendor: CustomVoiceVendor::ElevenLabs,
            inbound_custom_voice_vendor: CustomVoiceVendor::ElevenLabs,
            soniox: SonioxSettings::default(),
            elevenlabs: ElevenLabsSettings::default(),
            fishaudio: FishAudioSettings::default(),
            xai: XaiSettings::default(),
            artifacts_enabled: true,
            answer_language: String::new(),
            meeting_context: super::meeting_context::MeetingContextPayload::default(),
            custom_llm_profiles: Vec::new(),
            summary_custom_profile_id: None,
            custom_llm_api_keys: std::collections::HashMap::new(),
        }
    }
}

impl AppConfig {
    /// Seed per-mode stashes once for configs saved before mode prefs existed.
    fn seed_mode_stashes_if_empty(&mut self) {
        let langs_unseeded = self.interpreter_my_language.trim().is_empty()
            && self.interpreter_meeting_language.trim().is_empty()
            && self.notes_language.trim().is_empty();
        if langs_unseeded {
            if self.session_mode.is_notes() {
                self.notes_language = self.my_language.clone();
                self.interpreter_my_language = self.my_language.clone();
                // Active meeting was forced equal to my while Notes — recover a distinct
                // interpreter meeting lang when possible, else default English.
                self.interpreter_meeting_language = if self.meeting_language != self.my_language {
                    self.meeting_language.clone()
                } else {
                    "en".to_string()
                };
            } else {
                self.interpreter_my_language = self.my_language.clone();
                self.interpreter_meeting_language = self.meeting_language.clone();
                self.notes_language = self.my_language.clone();
            }
        } else {
            if self.interpreter_my_language.trim().is_empty() {
                self.interpreter_my_language = self.my_language.clone();
            }
            if self.interpreter_meeting_language.trim().is_empty() {
                self.interpreter_meeting_language =
                    if self.meeting_language != self.my_language || !self.session_mode.is_notes() {
                        self.meeting_language.clone()
                    } else {
                        "en".to_string()
                    };
            }
            if self.notes_language.trim().is_empty() {
                self.notes_language = self.my_language.clone();
            }
        }

        if self.interpreter_outbound_mode.is_none() {
            self.interpreter_outbound_mode = Some(if self.session_mode.is_notes() {
                PipelineOutputMode::Translated
            } else {
                self.outbound_mode
            });
        }
        if self.interpreter_inbound_mode.is_none() {
            self.interpreter_inbound_mode = Some(if self.session_mode.is_notes() {
                PipelineOutputMode::Translated
            } else {
                self.inbound_mode
            });
        }
        if self.interpreter_outbound_voice_output.is_none() {
            self.interpreter_outbound_voice_output = Some(if self.session_mode.is_notes() {
                OutboundVoiceOutput::ProviderNative
            } else {
                self.outbound_voice_output
            });
        }
        if self.interpreter_inbound_voice_output.is_none() {
            self.interpreter_inbound_voice_output = Some(if self.session_mode.is_notes() {
                InboundVoiceOutput::ProviderNative
            } else {
                self.inbound_voice_output
            });
        }
    }

    /// Project per-mode stashes into the active fields the runtime already reads.
    fn project_mode_stashes_to_active(&mut self) {
        if self.session_mode.is_notes() {
            let lang = self.notes_language.clone();
            self.my_language = lang.clone();
            self.meeting_language = lang;
            self.outbound_mode = PipelineOutputMode::OriginalAudio;
            self.inbound_mode = PipelineOutputMode::OriginalAudio;
            self.outbound_voice_output = OutboundVoiceOutput::ProviderNative;
            self.inbound_voice_output = InboundVoiceOutput::ProviderNative;
        } else {
            self.my_language = self.interpreter_my_language.clone();
            self.meeting_language = self.interpreter_meeting_language.clone();
            if let Some(mode) = self.interpreter_outbound_mode {
                self.outbound_mode = mode;
            }
            if let Some(mode) = self.interpreter_inbound_mode {
                self.inbound_mode = mode;
            }
            if let Some(voice) = self.interpreter_outbound_voice_output {
                self.outbound_voice_output = voice;
            }
            if let Some(voice) = self.interpreter_inbound_voice_output {
                self.inbound_voice_output = voice;
            }
        }
    }

    pub fn normalize(&mut self) {
        self.seed_mode_stashes_if_empty();
        self.project_mode_stashes_to_active();

        self.summary_provider =
            super::app_config_validate::normalize_summary_provider(self.summary_provider);
        // Defensive re-normalize (upsert validates strictly; load must not fail).
        for profile in &mut self.custom_llm_profiles {
            let _ = super::custom_llm::normalize_custom_llm_profile(profile);
        }
        // Drop in-memory keys whose profile no longer exists (load-time GC
        // mirrors the persisted GC in `config_store::stored`).
        self.custom_llm_api_keys
            .retain(|id, _| self.custom_llm_profiles.iter().any(|p| &p.id == id));
        let persisted = self.live_models_for_provider(self.ai_provider).to_vec();
        crate::ai::clamp_to_live_catalog(
            self.ai_provider,
            &persisted,
            &mut self.live_model,
            &mut self.my_language,
            &mut self.meeting_language,
        );
        // Keep stashes aligned with catalog-clamped active langs.
        if self.session_mode.is_notes() {
            self.notes_language = self.my_language.clone();
            self.meeting_language = self.my_language.clone();
        } else {
            self.interpreter_my_language = self.my_language.clone();
            self.interpreter_meeting_language = self.meeting_language.clone();
        }
        self.summary_model =
            normalize_summary_model_for_provider(self.summary_provider, &self.summary_model);
        self.vad_silence_duration_ms = self.vad_silence_duration_ms.clamp(100, 3000);
        self.auto_end_meeting_after_min =
            normalize_auto_end_meeting_after_min(self.auto_end_meeting_after_min);
        self.inbound_original_ducked_gain = self.inbound_original_ducked_gain.clamp(0.0, 0.5);
        if self.session_mode.is_notes() {
            // Gemini Live Translate cannot omit MT — clamp to a Notes-capable provider.
            if self.ai_provider == AiProvider::Gemini {
                self.ai_provider = AiProvider::Soniox;
                self.live_model = default_live_model_for_provider(AiProvider::Soniox);
            }
        }
        self.elevenlabs.elevenlabs_speed =
            crate::voice::config::clamp_elevenlabs_speed(self.elevenlabs.elevenlabs_speed);
        self.elevenlabs.elevenlabs_chunk_schedule_preset =
            crate::voice::config::normalize_chunk_schedule_preset(
                self.elevenlabs.elevenlabs_chunk_schedule_preset,
            );
        self.soniox.soniox_tts_voice =
            crate::providers::soniox::tts::config::normalize_soniox_tts_voice(
                &self.soniox.soniox_tts_voice,
            );
        if self.soniox.soniox_tts_outbound_voice.trim().is_empty() {
            self.soniox.soniox_tts_outbound_voice = self.soniox.soniox_tts_voice.clone();
        }
        self.soniox.soniox_tts_outbound_voice =
            crate::providers::soniox::tts::config::normalize_soniox_tts_voice(
                &self.soniox.soniox_tts_outbound_voice,
            );
        if self.soniox.soniox_tts_outbound_model.trim().is_empty() {
            self.soniox.soniox_tts_outbound_model =
                crate::config::soniox_settings::default_soniox_tts_model_field();
        }
        if self.soniox.soniox_tts_inbound_model.trim().is_empty() {
            self.soniox.soniox_tts_inbound_model = self.soniox.soniox_tts_outbound_model.clone();
        }
        self.soniox.soniox_tts_inbound_speed =
            crate::providers::soniox::tts::config::clamp_soniox_tts_speed(
                self.soniox.soniox_tts_inbound_speed,
            );
        self.soniox.soniox_tts_outbound_speed =
            crate::providers::soniox::tts::config::clamp_soniox_tts_speed(
                self.soniox.soniox_tts_outbound_speed,
            );
        if !self.soniox.soniox_tts_models.is_empty() {
            if !self
                .soniox
                .soniox_tts_models
                .iter()
                .any(|m| m.id == self.soniox.soniox_tts_outbound_model)
            {
                self.soniox.soniox_tts_outbound_model = self.soniox.soniox_tts_models[0].id.clone();
            }
            if !self
                .soniox
                .soniox_tts_models
                .iter()
                .any(|m| m.id == self.soniox.soniox_tts_inbound_model)
            {
                self.soniox.soniox_tts_inbound_model = self.soniox.soniox_tts_models[0].id.clone();
            }
        }
        self.xai.xai_outbound_speed =
            crate::providers::xai::config::clamp_speed(self.xai.xai_outbound_speed);
        self.xai.xai_inbound_speed =
            crate::providers::xai::config::clamp_speed(self.xai.xai_inbound_speed);
        self.fishaudio.fishaudio_outbound_speed = crate::providers::fishaudio::config::clamp_speed(
            self.fishaudio.fishaudio_outbound_speed,
        );
        self.fishaudio.fishaudio_inbound_speed = crate::providers::fishaudio::config::clamp_speed(
            self.fishaudio.fishaudio_inbound_speed,
        );
        self.fishaudio.fishaudio_outbound_top_p = crate::providers::fishaudio::config::clamp_top_p(
            self.fishaudio.fishaudio_outbound_top_p,
        );
        self.fishaudio.fishaudio_inbound_top_p = crate::providers::fishaudio::config::clamp_top_p(
            self.fishaudio.fishaudio_inbound_top_p,
        );
        if !self.soniox.soniox_tts_voices.is_empty()
            && !self
                .soniox
                .soniox_tts_voices
                .iter()
                .any(|v| v.id == self.soniox.soniox_tts_voice)
        {
            self.soniox.soniox_tts_voice = self.soniox.soniox_tts_voices[0].id.clone();
        }
        if !self.soniox.soniox_tts_voices.is_empty()
            && !self
                .soniox
                .soniox_tts_voices
                .iter()
                .any(|v| v.id == self.soniox.soniox_tts_outbound_voice)
        {
            self.soniox.soniox_tts_outbound_voice = self.soniox.soniox_tts_voices[0].id.clone();
        }
        self.soniox.soniox_endpoint_latency_adjustment_level =
            crate::providers::soniox::config::normalize_endpoint_latency_level(
                self.soniox.soniox_endpoint_latency_adjustment_level,
            );
        self.soniox.soniox_endpoint_sensitivity =
            crate::providers::soniox::config::normalize_endpoint_sensitivity(
                self.soniox.soniox_endpoint_sensitivity,
            );
        self.soniox.soniox_max_endpoint_delay_ms =
            crate::providers::soniox::config::normalize_max_endpoint_delay_ms(
                self.soniox.soniox_max_endpoint_delay_ms,
            );
        self.overlay.normalize();
    }

    /// Watchdog idle auto-end threshold. `None` when the setting is off.
    pub fn auto_end_meeting_idle_ms(&self) -> Option<u64> {
        if !self.auto_end_meeting {
            return None;
        }
        Some(u64::from(self.auto_end_meeting_after_min) * 60_000)
    }

    pub fn active_api_key(&self) -> &str {
        self.api_key_for(self.ai_provider)
    }

    pub fn api_key_for(&self, provider: AiProvider) -> &str {
        match provider {
            AiProvider::Gemini => &self.gemini_api_key,
            AiProvider::OpenAi => &self.openai_api_key,
            AiProvider::Soniox => &self.soniox_api_key,
        }
    }

    pub fn is_api_key_configured(&self) -> bool {
        !self.active_api_key().trim().is_empty()
    }

    /// Meeting → You mix of ducked original under translation is active.
    pub fn inbound_ducking_enabled(&self) -> bool {
        self.inbound_original_under_translation && self.inbound_original_ducked_gain > 0.0
    }

    pub fn is_soniox_api_key_configured(&self) -> bool {
        !self.soniox_api_key.trim().is_empty()
    }

    /// Soniox inbound TTS language follows Translate → You speak (`my_language`).
    pub fn resolve_soniox_tts_language(&self) -> &str {
        let lang = self.my_language.trim();
        if lang.is_empty() {
            "en"
        } else {
            lang
        }
    }

    /// Preferred AI output language for meeting summaries.
    /// `answer_language` set → that; else the meeting's "You"
    /// language; else `my_language`; final fallback English so the model
    /// always gets a concrete code.
    pub fn resolve_answer_language(&self, meeting_my_language: &str) -> String {
        let explicit = self.answer_language.trim();
        if !explicit.is_empty() {
            return explicit.to_string();
        }
        let meeting_lang = meeting_my_language.trim();
        if !meeting_lang.is_empty() {
            return meeting_lang.to_string();
        }
        let mine = self.my_language.trim();
        if mine.is_empty() {
            "en".to_string()
        } else {
            mine.to_string()
        }
    }

    pub fn live_models_for_provider(&self, provider: AiProvider) -> &[crate::ai::LiveModelOption] {
        match provider {
            AiProvider::Gemini => &self.gemini_live_models,
            AiProvider::OpenAi => &self.openai_live_models,
            AiProvider::Soniox => &self.soniox_live_models,
        }
    }

    pub fn is_elevenlabs_api_key_configured(&self) -> bool {
        !self.elevenlabs.elevenlabs_api_key.trim().is_empty()
    }

    pub fn is_fishaudio_api_key_configured(&self) -> bool {
        !self.fishaudio.fishaudio_api_key.trim().is_empty()
    }

    pub fn is_xai_api_key_configured(&self) -> bool {
        !self.xai.xai_api_key.trim().is_empty()
    }

    pub fn needs_custom_tts_for_outbound(&self) -> bool {
        self.outbound_voice_output.uses_custom_tts()
            && self.outbound_mode == PipelineOutputMode::Translated
    }

    pub fn needs_custom_tts_for_inbound(&self) -> bool {
        self.inbound_voice_output.uses_custom_tts()
            && self.inbound_mode == PipelineOutputMode::Translated
    }

    pub fn needs_elevenlabs_for_outbound(&self) -> bool {
        self.needs_custom_tts_for_outbound()
    }

    pub fn needs_elevenlabs_for_inbound(&self) -> bool {
        self.needs_custom_tts_for_inbound()
    }

    /// True when any summary chat LLM (built-in or custom profile) is usable.
    pub fn summary_fallback_available(&self) -> bool {
        self.summary_llm_selection().is_some()
    }

    pub fn summary_provider_and_key(&self) -> Option<(AiProvider, &str)> {
        match self.summary_provider {
            AiProvider::Gemini if !self.gemini_api_key.trim().is_empty() => {
                Some((AiProvider::Gemini, self.gemini_api_key.as_str()))
            }
            AiProvider::OpenAi if !self.openai_api_key.trim().is_empty() => {
                Some((AiProvider::OpenAi, self.openai_api_key.as_str()))
            }
            _ => None,
        }
    }

    /// Resolve the chat-LLM selection: the custom profile when
    /// `summary_custom_profile_id` points at an existing profile, else the
    /// built-in `summary_provider` + key. `None` when neither is usable —
    /// gates keep surfacing the existing "add a key" error. A deleted
    /// profile falls back to built-in with a warning (never crashes).
    pub fn summary_llm_selection(&self) -> Option<super::custom_llm::LlmSelection> {
        if let Some(profile_id) = self.summary_custom_profile_id.as_deref() {
            match self.custom_llm_profiles.iter().find(|p| p.id == profile_id) {
                Some(profile) => {
                    let api_key = self.custom_llm_api_keys.get(&profile.id).cloned();
                    return Some(super::custom_llm::LlmSelection::Custom {
                        profile: profile.clone(),
                        api_key,
                    });
                }
                None => {
                    tracing::warn!(
                        profile_id,
                        "summary custom profile missing — falling back to built-in provider"
                    );
                }
            }
        }
        let (provider, key) = self.summary_provider_and_key()?;
        Some(super::custom_llm::LlmSelection::BuiltIn {
            provider,
            api_key: key.to_string(),
            model: self.summary_model.clone(),
        })
    }

    /// Look up a custom profile by id.
    pub fn custom_llm_profile(&self, id: &str) -> Option<&super::custom_llm::CustomLlmProfile> {
        self.custom_llm_profiles.iter().find(|p| p.id == id)
    }

    /// IPC guard: structured artifacts panel.
    pub fn ensure_artifacts_enabled(&self) -> Result<(), String> {
        if self.artifacts_enabled {
            Ok(())
        } else {
            Err("feature_disabled: artifacts".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_artifacts_enabled_ok_when_enabled() {
        let cfg = AppConfig::default();
        assert!(cfg.ensure_artifacts_enabled().is_ok());
    }

    #[test]
    fn ensure_artifacts_enabled_rejects_with_feature_disabled_prefix() {
        let cfg = AppConfig {
            artifacts_enabled: false,
            ..Default::default()
        };
        assert_eq!(
            cfg.ensure_artifacts_enabled().unwrap_err(),
            "feature_disabled: artifacts"
        );
    }
}
