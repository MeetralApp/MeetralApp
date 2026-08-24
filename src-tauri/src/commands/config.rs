use serde::Deserialize;

use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::ai::AiProvider;
use crate::app_state::sync_app_state_after_config_change;
use crate::config::{
    AppConfig, ConfigView, DeviceRef, InboundVoiceOutput, OutboundVoiceOutput, OverlaySettings,
    PipelineOutputMode, SessionMode, ThemePreference, TranscriptLayout, VadSensitivity,
};
use crate::config_store;
use crate::runtime::engine::SharedEngine;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfigRequest {
    #[serde(default)]
    pub ai_provider: AiProvider,
    pub gemini_api_key: String,
    #[serde(default)]
    pub clear_gemini_api_key: bool,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub clear_openai_api_key: bool,
    #[serde(default)]
    pub soniox_api_key: String,
    #[serde(default)]
    pub clear_soniox_api_key: bool,
    pub my_language: String,
    pub meeting_language: String,
    #[serde(default)]
    pub session_mode: SessionMode,
    #[serde(default)]
    pub interpreter_my_language: Option<String>,
    #[serde(default)]
    pub interpreter_meeting_language: Option<String>,
    #[serde(default)]
    pub notes_language: Option<String>,
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
    #[serde(default, alias = "geminiModel")]
    pub live_model: String,
    #[serde(default)]
    pub gemini_live_models: Option<Vec<crate::ai::LiveModelOption>>,
    #[serde(default)]
    pub openai_live_models: Option<Vec<crate::ai::LiveModelOption>>,
    #[serde(default)]
    pub soniox_live_models: Option<Vec<crate::ai::LiveModelOption>>,
    #[serde(default, alias = "geminiSummaryModel")]
    pub summary_model: String,
    #[serde(default = "default_summary_provider")]
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
    #[serde(default, alias = "saveMeetingAudio")]
    pub record_meeting_audio: bool,
    #[serde(default, alias = "saveMeetingAudioFolder")]
    pub meeting_audio_save_folder: Option<String>,
    #[serde(default)]
    pub transcript_layout: TranscriptLayout,
    #[serde(default)]
    pub overlay: Option<OverlaySettings>,
    #[serde(default)]
    pub outbound_voice_output: OutboundVoiceOutput,
    #[serde(default)]
    pub inbound_voice_output: InboundVoiceOutput,
    #[serde(default)]
    pub outbound_custom_voice_vendor: crate::config::CustomVoiceVendor,
    #[serde(default)]
    pub inbound_custom_voice_vendor: crate::config::CustomVoiceVendor,
    #[serde(default)]
    pub soniox_always_on: Option<crate::providers::shared::live::SonioxContextPayload>,
    #[serde(default)]
    pub soniox_context_profiles: Option<Vec<crate::providers::shared::live::SonioxContextProfile>>,
    /// `None` = leave unchanged; `Some("")` = Always-on only; `Some(id)` = select profile.
    #[serde(default)]
    pub soniox_active_context_profile_id: Option<String>,
    #[serde(default)]
    pub soniox_tts_voice: Option<String>,
    #[serde(default)]
    pub soniox_tts_outbound_voice: Option<String>,
    #[serde(default)]
    pub soniox_tts_model: Option<String>,
    #[serde(default)]
    pub soniox_tts_voices: Option<Vec<crate::voice::SonioxVoiceOption>>,
    #[serde(default)]
    pub soniox_tts_models: Option<Vec<crate::voice::SonioxTtsModelOption>>,
    #[serde(default)]
    pub soniox_tts_inbound_speed: Option<f32>,
    #[serde(default)]
    pub soniox_tts_outbound_speed: Option<f32>,
    #[serde(default)]
    pub soniox_endpoint_latency_adjustment_level: Option<u8>,
    #[serde(default)]
    pub soniox_endpoint_sensitivity: Option<f64>,
    #[serde(default)]
    pub soniox_max_endpoint_delay_ms: Option<u32>,
    #[serde(default)]
    pub elevenlabs_api_key: String,
    #[serde(default)]
    pub clear_elevenlabs_api_key: bool,
    #[serde(default)]
    pub elevenlabs_voice_id: String,
    #[serde(default)]
    pub elevenlabs_voices: Option<Vec<crate::voice::ElevenLabsVoiceOption>>,
    #[serde(default)]
    pub elevenlabs_models: Option<Vec<crate::voice::ElevenLabsModelOption>>,
    #[serde(default)]
    pub elevenlabs_tts_model: String,
    #[serde(default)]
    pub elevenlabs_stability: Option<f32>,
    #[serde(default)]
    pub elevenlabs_similarity_boost: Option<f32>,
    #[serde(default)]
    pub elevenlabs_speed: Option<f32>,
    #[serde(default)]
    pub elevenlabs_use_speaker_boost: Option<bool>,
    #[serde(default)]
    pub elevenlabs_chunk_schedule_preset:
        Option<crate::voice::config::ElevenLabsChunkSchedulePreset>,
    #[serde(default)]
    pub elevenlabs_tts_language_auto: Option<bool>,
    #[serde(default)]
    pub elevenlabs_tts_language_code: Option<String>,
    #[serde(default)]
    pub elevenlabs_tts_synthesis_mode: Option<crate::voice::config::TtsSynthesisMode>,
    #[serde(default)]
    pub elevenlabs_playback_crossfade: Option<bool>,
    #[serde(default)]
    pub elevenlabs_crossfade_ms: Option<u32>,
    #[serde(default)]
    pub elevenlabs_inbound_voice_id: String,
    #[serde(default)]
    pub elevenlabs_inbound_tts_model: Option<String>,
    #[serde(default)]
    pub elevenlabs_inbound_stability: Option<f32>,
    #[serde(default)]
    pub elevenlabs_inbound_similarity_boost: Option<f32>,
    #[serde(default)]
    pub elevenlabs_inbound_tts_synthesis_mode: Option<crate::voice::config::TtsSynthesisMode>,
    #[serde(default)]
    pub fishaudio_api_key: String,
    #[serde(default)]
    pub clear_fishaudio_api_key: bool,
    #[serde(default)]
    pub fishaudio_voice_id: String,
    #[serde(default)]
    pub fishaudio_inbound_voice_id: String,
    #[serde(default)]
    pub fishaudio_voices: Option<Vec<crate::providers::fishaudio::FishAudioVoiceOption>>,
    #[serde(default)]
    pub fishaudio_models: Option<Vec<crate::providers::fishaudio::FishAudioModelOption>>,
    #[serde(default)]
    pub fishaudio_tts_model: String,
    #[serde(default)]
    pub fishaudio_inbound_tts_model: String,
    #[serde(default)]
    pub fishaudio_latency: Option<crate::config::FishAudioLatency>,
    #[serde(default)]
    pub fishaudio_inbound_latency: Option<crate::config::FishAudioLatency>,
    #[serde(default)]
    pub fishaudio_temperature: Option<f32>,
    #[serde(default)]
    pub fishaudio_inbound_temperature: Option<f32>,
    #[serde(default)]
    pub fishaudio_speed: Option<f32>,
    #[serde(default)]
    pub fishaudio_top_p: Option<f32>,
    #[serde(default)]
    pub xai_api_key: String,
    #[serde(default)]
    pub clear_xai_api_key: bool,
    #[serde(default)]
    pub xai_voice_id: String,
    #[serde(default)]
    pub xai_inbound_voice_id: String,
    #[serde(default)]
    pub xai_voices: Option<Vec<crate::providers::xai::XaiVoiceOption>>,
    #[serde(default)]
    pub xai_latency: Option<crate::config::XaiLatency>,
    #[serde(default)]
    pub xai_inbound_latency: Option<crate::config::XaiLatency>,
    #[serde(default)]
    pub xai_speed: Option<f32>,
    #[serde(default = "default_true")]
    pub artifacts_enabled: bool,
    #[serde(default)]
    pub answer_language: String,
    /// `None` = leave unchanged; `Some(payload)` = replace the app-level
    /// meeting context for summary prompts.
    #[serde(default)]
    pub meeting_context: Option<crate::config::MeetingContextPayload>,
    /// `None` = leave unchanged; `Some("")` = use built-in provider;
    /// `Some(id)` = select this custom profile.
    #[serde(default)]
    pub summary_custom_profile_id: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_inbound_original_ducked_gain() -> f32 {
    0.18
}

fn default_summary_provider() -> AiProvider {
    AiProvider::Gemini
}

fn default_vad_silence() -> u32 {
    800
}

impl SaveConfigRequest {
    fn into_app_config(self, existing: &AppConfig) -> AppConfig {
        let gemini_api_key = if self.clear_gemini_api_key {
            String::new()
        } else if self.gemini_api_key.trim().is_empty() {
            existing.gemini_api_key.clone()
        } else {
            self.gemini_api_key
        };

        let openai_api_key = if self.clear_openai_api_key {
            String::new()
        } else if self.openai_api_key.trim().is_empty() {
            existing.openai_api_key.clone()
        } else {
            self.openai_api_key
        };

        let soniox_api_key = if self.clear_soniox_api_key {
            String::new()
        } else if self.soniox_api_key.trim().is_empty() {
            existing.soniox_api_key.clone()
        } else {
            self.soniox_api_key
        };

        let elevenlabs_api_key = if self.clear_elevenlabs_api_key {
            String::new()
        } else if self.elevenlabs_api_key.trim().is_empty() {
            existing.elevenlabs.elevenlabs_api_key.clone()
        } else {
            self.elevenlabs_api_key
        };

        let fishaudio_api_key = if self.clear_fishaudio_api_key {
            String::new()
        } else if self.fishaudio_api_key.trim().is_empty() {
            existing.fishaudio.fishaudio_api_key.clone()
        } else {
            self.fishaudio_api_key
        };

        let xai_api_key = if self.clear_xai_api_key {
            String::new()
        } else if self.xai_api_key.trim().is_empty() {
            existing.xai.xai_api_key.clone()
        } else {
            self.xai_api_key
        };

        let mut config = AppConfig {
            ai_provider: self.ai_provider,
            gemini_api_key,
            openai_api_key,
            soniox_api_key,
            my_language: self.my_language,
            meeting_language: self.meeting_language,
            session_mode: self.session_mode,
            interpreter_my_language: self
                .interpreter_my_language
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| existing.interpreter_my_language.clone()),
            interpreter_meeting_language: self
                .interpreter_meeting_language
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| existing.interpreter_meeting_language.clone()),
            notes_language: self
                .notes_language
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| existing.notes_language.clone()),
            interpreter_outbound_mode: self
                .interpreter_outbound_mode
                .or(existing.interpreter_outbound_mode),
            interpreter_inbound_mode: self
                .interpreter_inbound_mode
                .or(existing.interpreter_inbound_mode),
            interpreter_outbound_voice_output: self
                .interpreter_outbound_voice_output
                .or(existing.interpreter_outbound_voice_output),
            interpreter_inbound_voice_output: self
                .interpreter_inbound_voice_output
                .or(existing.interpreter_inbound_voice_output),
            user_mic: self.user_mic,
            teams_mic_feed: self.teams_mic_feed,
            meeting_capture: self.meeting_capture,
            local_playback: self.local_playback,
            outbound_mode: self.outbound_mode,
            inbound_mode: self.inbound_mode,

            live_model: if self.live_model.trim().is_empty() {
                existing.live_model.clone()
            } else {
                self.live_model
            },

            gemini_live_models: self
                .gemini_live_models
                .unwrap_or_else(|| existing.gemini_live_models.clone()),
            openai_live_models: self
                .openai_live_models
                .unwrap_or_else(|| existing.openai_live_models.clone()),
            soniox_live_models: if self.clear_soniox_api_key {
                Vec::new()
            } else {
                self.soniox_live_models
                    .unwrap_or_else(|| existing.soniox_live_models.clone())
            },

            summary_model: if self.summary_model.trim().is_empty() {
                existing.summary_model.clone()
            } else {
                self.summary_model
            },

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
            meeting_audio_save_folder: self
                .meeting_audio_save_folder
                .clone()
                .unwrap_or_else(|| existing.meeting_audio_save_folder.clone()),
            transcript_layout: self.transcript_layout,
            overlay: {
                let mut overlay = self.overlay.unwrap_or_else(|| existing.overlay.clone());
                overlay.normalize();
                overlay
            },
            outbound_voice_output: self.outbound_voice_output,
            inbound_voice_output: self.inbound_voice_output,
            outbound_custom_voice_vendor: self.outbound_custom_voice_vendor,
            inbound_custom_voice_vendor: self.inbound_custom_voice_vendor,
            soniox: crate::config::SonioxSettings {
                soniox_always_on: self
                    .soniox_always_on
                    .unwrap_or_else(|| existing.soniox.soniox_always_on.clone()),
                soniox_context_profiles: self
                    .soniox_context_profiles
                    .unwrap_or_else(|| existing.soniox.soniox_context_profiles.clone()),
                soniox_active_context_profile_id: match self.soniox_active_context_profile_id {
                    None => existing.soniox.soniox_active_context_profile_id.clone(),
                    Some(id) if id.trim().is_empty() => None,
                    Some(id) => Some(id),
                },
                soniox_tts_voice: self
                    .soniox_tts_voice
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(|| existing.soniox.soniox_tts_voice.clone()),
                soniox_tts_outbound_voice: self
                    .soniox_tts_outbound_voice
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(|| existing.soniox.soniox_tts_outbound_voice.clone()),
                soniox_tts_model: self
                    .soniox_tts_model
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(|| existing.soniox.soniox_tts_model.clone()),
                soniox_tts_voices: if self.clear_soniox_api_key {
                    Vec::new()
                } else {
                    self.soniox_tts_voices
                        .unwrap_or_else(|| existing.soniox.soniox_tts_voices.clone())
                },
                soniox_tts_models: if self.clear_soniox_api_key {
                    Vec::new()
                } else {
                    self.soniox_tts_models
                        .unwrap_or_else(|| existing.soniox.soniox_tts_models.clone())
                },
                soniox_tts_inbound_speed: self
                    .soniox_tts_inbound_speed
                    .unwrap_or(existing.soniox.soniox_tts_inbound_speed),
                soniox_tts_outbound_speed: self
                    .soniox_tts_outbound_speed
                    .unwrap_or(existing.soniox.soniox_tts_outbound_speed),
                soniox_endpoint_latency_adjustment_level: self
                    .soniox_endpoint_latency_adjustment_level
                    .unwrap_or(existing.soniox.soniox_endpoint_latency_adjustment_level),
                soniox_endpoint_sensitivity: self
                    .soniox_endpoint_sensitivity
                    .unwrap_or(existing.soniox.soniox_endpoint_sensitivity),
                soniox_max_endpoint_delay_ms: self
                    .soniox_max_endpoint_delay_ms
                    .unwrap_or(existing.soniox.soniox_max_endpoint_delay_ms),
            },
            elevenlabs: crate::config::ElevenLabsSettings {
                elevenlabs_api_key,
                elevenlabs_voice_id: if self.elevenlabs_voice_id.trim().is_empty() {
                    existing.elevenlabs.elevenlabs_voice_id.clone()
                } else {
                    self.elevenlabs_voice_id
                },
                elevenlabs_voices: if self.clear_elevenlabs_api_key {
                    Vec::new()
                } else {
                    self.elevenlabs_voices
                        .unwrap_or_else(|| existing.elevenlabs.elevenlabs_voices.clone())
                },
                elevenlabs_models: if self.clear_elevenlabs_api_key {
                    Vec::new()
                } else {
                    self.elevenlabs_models
                        .unwrap_or_else(|| existing.elevenlabs.elevenlabs_models.clone())
                },
                elevenlabs_tts_model: if self.elevenlabs_tts_model.trim().is_empty() {
                    existing.elevenlabs.elevenlabs_tts_model.clone()
                } else {
                    self.elevenlabs_tts_model
                },
                elevenlabs_stability: self
                    .elevenlabs_stability
                    .unwrap_or(existing.elevenlabs.elevenlabs_stability),
                elevenlabs_similarity_boost: self
                    .elevenlabs_similarity_boost
                    .unwrap_or(existing.elevenlabs.elevenlabs_similarity_boost),
                elevenlabs_speed: self
                    .elevenlabs_speed
                    .unwrap_or(existing.elevenlabs.elevenlabs_speed),
                elevenlabs_use_speaker_boost: self
                    .elevenlabs_use_speaker_boost
                    .unwrap_or(existing.elevenlabs.elevenlabs_use_speaker_boost),
                elevenlabs_chunk_schedule_preset: self
                    .elevenlabs_chunk_schedule_preset
                    .unwrap_or(existing.elevenlabs.elevenlabs_chunk_schedule_preset),
                elevenlabs_tts_synthesis_mode: self
                    .elevenlabs_tts_synthesis_mode
                    .unwrap_or(existing.elevenlabs.elevenlabs_tts_synthesis_mode),
                elevenlabs_tts_language_auto: self
                    .elevenlabs_tts_language_auto
                    .unwrap_or(existing.elevenlabs.elevenlabs_tts_language_auto),
                elevenlabs_tts_language_code: self
                    .elevenlabs_tts_language_code
                    .clone()
                    .unwrap_or_else(|| existing.elevenlabs.elevenlabs_tts_language_code.clone()),
                elevenlabs_auto_mode: existing.elevenlabs.elevenlabs_auto_mode,
                unified_outbound_topology: existing.elevenlabs.unified_outbound_topology,
                elevenlabs_playback_crossfade: self
                    .elevenlabs_playback_crossfade
                    .unwrap_or(existing.elevenlabs.elevenlabs_playback_crossfade),
                elevenlabs_crossfade_ms: self
                    .elevenlabs_crossfade_ms
                    .unwrap_or(existing.elevenlabs.elevenlabs_crossfade_ms),
                elevenlabs_inbound_voice_id: if self.elevenlabs_inbound_voice_id.trim().is_empty() {
                    existing.elevenlabs.elevenlabs_inbound_voice_id.clone()
                } else {
                    self.elevenlabs_inbound_voice_id
                },
                elevenlabs_inbound_tts_model: self
                    .elevenlabs_inbound_tts_model
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(|| existing.elevenlabs.elevenlabs_inbound_tts_model.clone()),
                elevenlabs_inbound_stability: self
                    .elevenlabs_inbound_stability
                    .unwrap_or(existing.elevenlabs.elevenlabs_inbound_stability),
                elevenlabs_inbound_similarity_boost: self
                    .elevenlabs_inbound_similarity_boost
                    .unwrap_or(existing.elevenlabs.elevenlabs_inbound_similarity_boost),
                elevenlabs_inbound_tts_synthesis_mode: self
                    .elevenlabs_inbound_tts_synthesis_mode
                    .unwrap_or(existing.elevenlabs.elevenlabs_inbound_tts_synthesis_mode),
            },
            fishaudio: crate::config::FishAudioSettings {
                fishaudio_api_key,
                fishaudio_voice_id: if self.fishaudio_voice_id.trim().is_empty() {
                    existing.fishaudio.fishaudio_voice_id.clone()
                } else {
                    self.fishaudio_voice_id
                },
                fishaudio_inbound_voice_id: if self.fishaudio_inbound_voice_id.trim().is_empty() {
                    existing.fishaudio.fishaudio_inbound_voice_id.clone()
                } else {
                    self.fishaudio_inbound_voice_id
                },
                fishaudio_voices: if self.clear_fishaudio_api_key {
                    Vec::new()
                } else {
                    self.fishaudio_voices
                        .unwrap_or_else(|| existing.fishaudio.fishaudio_voices.clone())
                },
                fishaudio_models: if self.clear_fishaudio_api_key {
                    Vec::new()
                } else {
                    self.fishaudio_models
                        .unwrap_or_else(|| existing.fishaudio.fishaudio_models.clone())
                },
                fishaudio_tts_model: if self.fishaudio_tts_model.trim().is_empty() {
                    existing.fishaudio.fishaudio_tts_model.clone()
                } else {
                    self.fishaudio_tts_model
                },
                fishaudio_inbound_tts_model: if self.fishaudio_inbound_tts_model.trim().is_empty() {
                    existing.fishaudio.fishaudio_inbound_tts_model.clone()
                } else {
                    self.fishaudio_inbound_tts_model
                },
                fishaudio_latency: self
                    .fishaudio_latency
                    .unwrap_or(existing.fishaudio.fishaudio_latency),
                fishaudio_inbound_latency: self
                    .fishaudio_inbound_latency
                    .unwrap_or(existing.fishaudio.fishaudio_inbound_latency),
                fishaudio_temperature: self
                    .fishaudio_temperature
                    .unwrap_or(existing.fishaudio.fishaudio_temperature),
                fishaudio_inbound_temperature: self
                    .fishaudio_inbound_temperature
                    .unwrap_or(existing.fishaudio.fishaudio_inbound_temperature),
                fishaudio_speed: self
                    .fishaudio_speed
                    .unwrap_or(existing.fishaudio.fishaudio_speed),
                fishaudio_top_p: self
                    .fishaudio_top_p
                    .unwrap_or(existing.fishaudio.fishaudio_top_p),
            },
            xai: crate::config::XaiSettings {
                xai_api_key,
                xai_voice_id: if self.xai_voice_id.trim().is_empty() {
                    existing.xai.xai_voice_id.clone()
                } else {
                    self.xai_voice_id
                },
                xai_inbound_voice_id: if self.xai_inbound_voice_id.trim().is_empty() {
                    existing.xai.xai_inbound_voice_id.clone()
                } else {
                    self.xai_inbound_voice_id
                },
                xai_voices: if self.clear_xai_api_key {
                    Vec::new()
                } else {
                    self.xai_voices
                        .unwrap_or_else(|| existing.xai.xai_voices.clone())
                },
                xai_latency: self.xai_latency.unwrap_or(existing.xai.xai_latency),
                xai_inbound_latency: self
                    .xai_inbound_latency
                    .unwrap_or(existing.xai.xai_inbound_latency),
                xai_speed: self.xai_speed.unwrap_or(existing.xai.xai_speed),
            },
            artifacts_enabled: self.artifacts_enabled,
            answer_language: self.answer_language,
            meeting_context: self
                .meeting_context
                .unwrap_or_else(|| existing.meeting_context.clone()),
            // Profiles are managed via dedicated CRUD commands, never via
            // save_config — keep existing.
            custom_llm_profiles: existing.custom_llm_profiles.clone(),
            summary_custom_profile_id: match self.summary_custom_profile_id {
                None => existing.summary_custom_profile_id.clone(),
                Some(id) if id.trim().is_empty() => None,
                Some(id) => Some(id),
            },
            custom_llm_api_keys: existing.custom_llm_api_keys.clone(),
        };

        config.normalize();

        config
    }
}

#[tauri::command]

pub async fn get_config(store: State<'_, AsyncMutex<AppConfig>>) -> Result<ConfigView, String> {
    let config = store.lock().await;

    Ok(ConfigView::from(&*config))
}

#[tauri::command]

pub async fn save_config(
    app: AppHandle,

    config: SaveConfigRequest,

    store: State<'_, AsyncMutex<AppConfig>>,

    engine: State<'_, SharedEngine>,
) -> Result<crate::runtime::engine::AudioDevicesApplyResult, String> {
    let (previous, merged) = {
        let mut guard = store.lock().await;
        let previous = guard.clone();
        let merged = config.into_app_config(&guard);
        *guard = merged.clone();
        (previous, merged)
    };

    if let Some(runtime) = app.try_state::<crate::tray::RuntimeConfig>() {
        runtime.set_close_to_tray(merged.close_to_tray);
    }

    let merged_for_disk = merged.clone();
    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || config_store::save(&app_bg, &merged_for_disk))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;

    if let Err(e) = crate::overlay::apply_settings(&app, &merged.overlay) {
        tracing::warn!("overlay apply after save_config failed: {e}");
    }

    let devices = crate::audio::list_devices_async()
        .await
        .map_err(|e| e.to_string())?;
    let engine_bg = engine.inner().clone();
    let mut guard = engine.lock().await;
    guard
        .apply_audio_devices_after_config_change(&previous, &merged, &devices, &app, engine_bg)
        .await
}

// ---------------------------------------------------------------------------
// Custom OpenAI-compatible LLM profiles
// ---------------------------------------------------------------------------

/// Public profile shape for the FE — never carries the API key.
pub type CustomLlmProfileView = crate::config::CustomLlmProfileView;

fn profile_view(
    config: &AppConfig,
    profile: &crate::config::CustomLlmProfile,
) -> CustomLlmProfileView {
    CustomLlmProfileView {
        api_key_configured: config
            .custom_llm_api_keys
            .get(&profile.id)
            .is_some_and(|k| !k.trim().is_empty()),
        profile: profile.clone(),
    }
}

#[tauri::command]
pub async fn list_custom_llm_profiles(
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<CustomLlmProfileView>, String> {
    let guard = store.lock().await;
    Ok(guard
        .custom_llm_profiles
        .iter()
        .map(|p| profile_view(&guard, p))
        .collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomLlmProfileInput {
    /// `None` = create (id generated); `Some(id)` = edit in place.
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    pub base_url: String,
    pub chat_model: String,
    /// `None` = keep existing key (edit) / no key (create);
    /// `Some("")` = clear the key; `Some(key)` = set it.
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Mandatory validation before a profile may be saved/used: one small
/// JSON-mode generate (summary prefers `response_format`) AND one streaming
/// generate. Streaming is hard-required. JSON mode soft-passes when the upstream
/// rejects `response_format` (common on OpenRouter/Anthropic) — callers then
/// use prompt-only JSON.
///
/// Returns whether wire `response_format: json_object` is supported.
async fn test_custom_llm_profile_calls(
    profile: &crate::config::CustomLlmProfile,
    api_key: Option<&str>,
) -> Result<bool, String> {
    use crate::ai::llm::{ChatLlmProvider, ChatRequest};

    const JSON_PROBE_PROMPT: &str = r#"Reply with JSON only: {"ok":true}"#;

    let chat = crate::providers::compatible::chat::CompatibleChatLlm::new(
        &profile.base_url,
        api_key,
        &profile.chat_model,
        true,
    );
    let json_req = ChatRequest::plain(&profile.chat_model, None, JSON_PROBE_PROMPT, true);
    let json_mode = match chat.generate(&json_req).await {
        Ok(_) => true,
        Err(err) if is_json_mode_soft_fail_candidate(&err) => {
            let plain_req = ChatRequest::plain(&profile.chat_model, None, JSON_PROBE_PROMPT, false);
            chat.generate(&plain_req).await.map_err(|e| {
                format!("Chat test failed (JSON mode unsupported, plain retry also failed): {e:#}")
            })?;
            tracing::info!(
                model = %profile.chat_model,
                base_url = %profile.base_url,
                "custom LLM JSON mode unsupported — saving with prompt-only JSON"
            );
            false
        }
        Err(err) => {
            return Err(format!("Chat test (JSON mode) failed: {err:#}"));
        }
    };

    let stream_req = ChatRequest::plain(
        &profile.chat_model,
        None,
        "Reply with the single word OK.",
        false,
    );
    chat.generate_stream(&stream_req, Box::new(|_| {}))
        .await
        .map_err(|e| format!("Chat test (streaming) failed: {e:#}"))?;

    Ok(json_mode)
}

/// OpenRouter/Anthropic often reject `response_format` with InvalidRequest /
/// `invalid_parameter_error`. Those are soft-fail candidates for the JSON
/// probe; auth/unavailable/timeout stay hard failures.
fn is_json_mode_soft_fail_candidate(err: &anyhow::Error) -> bool {
    use crate::ai::llm::{LlmError, LlmErrorKind};
    if let Some(llm) = err.downcast_ref::<LlmError>() {
        // Only InvalidRequest is eligible; Auth/Timeout/Unavailable hard-fail.
        return llm.kind == LlmErrorKind::InvalidRequest;
    }
    let msg = format!("{err:#}").to_lowercase();
    msg.contains("invalid_parameter")
        || msg.contains("response_format")
        || msg.contains("json_object")
        || msg.contains("json mode")
}

/// Resolve the effective API key for a custom-profile probe/upsert.
/// `Some(key)` wins; `Some("")` clears; `None` keeps the stored key when editing.
fn resolve_custom_llm_api_key(
    store: &AppConfig,
    profile_id: &str,
    request_key: Option<String>,
) -> Option<String> {
    match request_key {
        Some(key) if key.trim().is_empty() => None,
        Some(key) => Some(key),
        None => store.custom_llm_api_keys.get(profile_id).cloned(),
    }
}

/// Dry-run connection test (Settings "Test connection" button) — same calls
/// as the save gate, without persisting anything. Resolves a stored key when
/// editing (`id` set) and the form leaves `apiKey` empty.
#[tauri::command]
pub async fn test_custom_llm_profile(
    request: CustomLlmProfileInput,
    store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<(), String> {
    let mut profile = crate::config::CustomLlmProfile {
        id: request.id.clone().unwrap_or_else(|| "test".to_string()),
        label: request.label.clone(),
        base_url: request.base_url.clone(),
        chat_model: request.chat_model.clone(),
        json_mode: true,
    };
    crate::config::normalize_custom_llm_profile(&mut profile)?;
    let effective_key = {
        let guard = store.lock().await;
        resolve_custom_llm_api_key(&guard, &profile.id, request.api_key)
    };
    let _json_mode = test_custom_llm_profile_calls(&profile, effective_key.as_deref()).await?;
    Ok(())
}

#[tauri::command]
pub async fn upsert_custom_llm_profile(
    app: AppHandle,
    request: CustomLlmProfileInput,
    store: State<'_, AsyncMutex<AppConfig>>,
    engine: State<'_, SharedEngine>,
) -> Result<CustomLlmProfileView, String> {
    let mut profile = crate::config::CustomLlmProfile {
        id: request
            .id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        label: request.label.clone(),
        base_url: request.base_url.clone(),
        chat_model: request.chat_model.clone(),
        json_mode: true,
    };
    crate::config::normalize_custom_llm_profile(&mut profile)?;

    // Resolve the effective key BEFORE the test call: explicit set/clear wins,
    // otherwise keep the stored one (edit flow).
    let effective_key = {
        let guard = store.lock().await;
        resolve_custom_llm_api_key(&guard, &profile.id, request.api_key)
    };

    // Save gate: test call must pass BEFORE anything persists.
    // Soft-pass sets `json_mode = false` when `response_format` is rejected.
    profile.json_mode = test_custom_llm_profile_calls(&profile, effective_key.as_deref()).await?;

    let view = {
        let mut guard = store.lock().await;
        match guard
            .custom_llm_profiles
            .iter_mut()
            .find(|p| p.id == profile.id)
        {
            Some(existing) => *existing = profile.clone(),
            None => guard.custom_llm_profiles.push(profile.clone()),
        }
        match &effective_key {
            Some(key) => {
                guard
                    .custom_llm_api_keys
                    .insert(profile.id.clone(), key.clone());
            }
            None => {
                guard.custom_llm_api_keys.remove(&profile.id);
            }
        }
        let view = profile_view(&guard, &profile);
        let merged = guard.clone();
        drop(guard);
        let app_bg = app.clone();
        let merged_bg = merged.clone();
        tokio::task::spawn_blocking(move || config_store::save(&app_bg, &merged_bg))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;
        view
    };
    Ok(view)
}

#[tauri::command]
pub async fn delete_custom_llm_profile(
    app: AppHandle,
    id: String,
    store: State<'_, AsyncMutex<AppConfig>>,
    engine: State<'_, SharedEngine>,
) -> Result<(), String> {
    let account = {
        let mut guard = store.lock().await;
        let Some(index) = guard.custom_llm_profiles.iter().position(|p| p.id == id) else {
            return Err("Profile not found.".into());
        };
        let profile = guard.custom_llm_profiles.remove(index);
        guard.custom_llm_api_keys.remove(&id);
        // Selected profile deleted → selection falls back to built-in (the
        // resolver also warns; clearing here keeps config.json honest).
        if guard.summary_custom_profile_id.as_deref() == Some(id.as_str()) {
            guard.summary_custom_profile_id = None;
        }
        let account = profile.keychain_account();
        let merged = guard.clone();
        drop(guard);
        let app_bg = app.clone();
        let merged_bg = merged.clone();
        tokio::task::spawn_blocking(move || config_store::save(&app_bg, &merged_bg))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;
        account
    };
    // macOS keychain purge (Windows DPAPI ciphertext was dropped above).
    if let Err(error) = crate::secret::delete_for_account(&account) {
        tracing::warn!(profile_id = %id, error = %error, "custom LLM keychain delete failed");
    }
    Ok(())
}

#[cfg(test)]
mod custom_llm_probe_tests {
    use super::{is_json_mode_soft_fail_candidate, resolve_custom_llm_api_key};
    use crate::ai::llm::{LlmError, LlmErrorKind};
    use crate::config::AppConfig;
    use std::collections::HashMap;

    #[test]
    fn soft_fail_candidate_accepts_invalid_request() {
        let err: anyhow::Error =
            LlmError::new(LlmErrorKind::InvalidRequest, "invalid_parameter_error").into();
        assert!(is_json_mode_soft_fail_candidate(&err));
    }

    #[test]
    fn soft_fail_candidate_rejects_auth() {
        let err: anyhow::Error = LlmError::new(LlmErrorKind::Auth, "401 unauthorized").into();
        assert!(!is_json_mode_soft_fail_candidate(&err));
    }

    #[test]
    fn soft_fail_candidate_matches_response_format_in_message() {
        let err = anyhow::anyhow!("Compatible API error 400: response_format not supported");
        assert!(is_json_mode_soft_fail_candidate(&err));
    }

    #[test]
    fn resolve_key_keeps_stored_when_request_omits() {
        let config = AppConfig {
            custom_llm_api_keys: HashMap::from([("p1".into(), "stored-key".into())]),
            ..AppConfig::default()
        };
        assert_eq!(
            resolve_custom_llm_api_key(&config, "p1", None).as_deref(),
            Some("stored-key")
        );
    }

    #[test]
    fn resolve_key_explicit_set_and_clear() {
        let config = AppConfig {
            custom_llm_api_keys: HashMap::from([("p1".into(), "stored-key".into())]),
            ..AppConfig::default()
        };
        assert_eq!(
            resolve_custom_llm_api_key(&config, "p1", Some("new-key".into())).as_deref(),
            Some("new-key")
        );
        assert_eq!(
            resolve_custom_llm_api_key(&config, "p1", Some("  ".into())),
            None
        );
    }
}
