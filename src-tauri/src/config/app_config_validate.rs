use crate::ai::{is_supported_language_for_provider, AiProvider};
use crate::audio::{
    validate_audio_setup, validate_for_direct_inbound, validate_for_direct_outbound,
    validate_for_start_inbound, validate_for_start_outbound, AudioDeviceInfo, AudioSetupValidation,
};

use super::app_config::AppConfig;

/// Single normalization site for the summary-provider selector:
/// only providers with a native chat LLM are selectable; anything else
/// (Soniox, unknown persisted values) falls back to Gemini. Previously
/// duplicated in `commands/config.rs`, `config_store/stored.rs`, and
/// `AppConfig::normalize`.
pub(crate) fn normalize_summary_provider(provider: AiProvider) -> AiProvider {
    match provider {
        AiProvider::OpenAi => AiProvider::OpenAi,
        _ => AiProvider::Gemini,
    }
}

impl AppConfig {
    pub fn validate_custom_voice_outbound_setup(&self) -> Result<(), String> {
        if !self.needs_custom_tts_for_outbound() {
            return Ok(());
        }
        match self.outbound_custom_voice_vendor {
            crate::config::CustomVoiceVendor::ElevenLabs => {
                if !self.is_elevenlabs_api_key_configured() {
                    return Err(
                        "ElevenLabs API key is required for custom voice. Add it in Settings → Voice."
                            .into(),
                    );
                }
                if self.elevenlabs.elevenlabs_voice_id.trim().is_empty() {
                    return Err(
                        "ElevenLabs voice ID is required for You → Meeting custom voice. Add your PVC voice ID in Settings → Voice.".into(),
                    );
                }
            }
            crate::config::CustomVoiceVendor::FishAudio => {
                if !self.is_fishaudio_api_key_configured() {
                    return Err(
                        "Fish Audio API key is required for custom voice. Add it in Settings → Voice."
                            .into(),
                    );
                }
                if self.fishaudio.fishaudio_voice_id.trim().is_empty() {
                    return Err(
                        "Fish Audio voice ID is required for You → Meeting custom voice. Pick a voice in Settings → Voice.".into(),
                    );
                }
            }
            crate::config::CustomVoiceVendor::Xai => {
                if !self.is_xai_api_key_configured() {
                    return Err(
                        "xAI API key is required for custom voice. Add it in Settings → Voice."
                            .into(),
                    );
                }
                if self.xai.xai_voice_id.trim().is_empty() {
                    return Err(
                        "xAI voice ID is required for You → Meeting custom voice. Pick a voice in Settings → Voice.".into(),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn validate_custom_voice_inbound_setup(&self) -> Result<(), String> {
        if !self.needs_custom_tts_for_inbound() {
            return Ok(());
        }
        match self.inbound_custom_voice_vendor {
            crate::config::CustomVoiceVendor::ElevenLabs => {
                if !self.is_elevenlabs_api_key_configured() {
                    return Err(
                        "ElevenLabs API key is required for Meeting → You custom voice. Add it in Settings → Voice."
                            .into(),
                    );
                }
                if self
                    .elevenlabs
                    .elevenlabs_inbound_voice_id
                    .trim()
                    .is_empty()
                {
                    return Err(
                        "ElevenLabs voice ID is required for Meeting → You custom voice. Add a voice in Settings → Voice.".into(),
                    );
                }
            }
            crate::config::CustomVoiceVendor::FishAudio => {
                if !self.is_fishaudio_api_key_configured() {
                    return Err(
                        "Fish Audio API key is required for Meeting → You custom voice. Add it in Settings → Voice."
                            .into(),
                    );
                }
                if self.fishaudio.fishaudio_inbound_voice_id.trim().is_empty() {
                    return Err(
                        "Fish Audio voice ID is required for Meeting → You custom voice. Pick a voice in Settings → Voice.".into(),
                    );
                }
            }
            crate::config::CustomVoiceVendor::Xai => {
                if !self.is_xai_api_key_configured() {
                    return Err(
                        "xAI API key is required for Meeting → You custom voice. Add it in Settings → Voice."
                            .into(),
                    );
                }
                if self.xai.xai_inbound_voice_id.trim().is_empty() {
                    return Err(
                        "xAI voice ID is required for Meeting → You custom voice. Pick a voice in Settings → Voice.".into(),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn validate_elevenlabs_outbound_setup(&self) -> Result<(), String> {
        self.validate_custom_voice_outbound_setup()
    }

    pub fn validate_elevenlabs_inbound_setup(&self) -> Result<(), String> {
        self.validate_custom_voice_inbound_setup()
    }

    /// Validates both directions when either needs custom voice.
    pub fn validate_elevenlabs_setup(&self) -> Result<(), String> {
        self.validate_custom_voice_outbound_setup()?;
        self.validate_custom_voice_inbound_setup()
    }

    pub fn validate_for_start(&self) -> Result<(), String> {
        if !self.is_api_key_configured() {
            return Err(format!(
                "{} API key is required. Add it in Settings.",
                self.ai_provider.label()
            ));
        }
        if self.my_language.is_empty() || self.meeting_language.is_empty() {
            return Err("Please select both languages".into());
        }
        if !is_supported_language_for_provider(self.ai_provider, &self.my_language) {
            return Err(format!(
                "Your language is not supported by {}",
                self.ai_provider.label()
            ));
        }
        if !is_supported_language_for_provider(self.ai_provider, &self.meeting_language) {
            return Err(format!(
                "Meeting language is not supported by {}",
                self.ai_provider.label()
            ));
        }
        if self.ai_provider == AiProvider::Soniox {
            let resolved = crate::providers::soniox::resolve_active_soniox_context(
                &self.soniox.soniox_always_on,
                &self.soniox.soniox_context_profiles,
                self.soniox.soniox_active_context_profile_id.as_deref(),
            );
            if crate::providers::soniox::context_over_budget(&resolved) {
                return Err(
                    "Soniox context is over the character budget. Fix it in Settings → Translate → Soniox context."
                        .into(),
                );
            }
        }
        if self.session_mode.is_notes() {
            let caps = crate::capabilities::get_provider_catalog(self.ai_provider).capabilities;
            if !caps.supports_notes_stt_only {
                return Err(format!(
                    "Notes mode requires Soniox or OpenAI (STT-only). {} cannot omit MT — switch provider, or use Interpreter.",
                    self.ai_provider.label()
                ));
            }
            if self.my_language != self.meeting_language {
                return Err(
                    "Notes mode uses one language. Set Meeting language to match Yours, or switch to Interpreter."
                        .into(),
                );
            }
        }
        Ok(())
    }

    pub fn validate_audio_setup(&self, devices: &[AudioDeviceInfo]) -> AudioSetupValidation {
        validate_audio_setup(self, devices)
    }

    pub fn validate_for_start_outbound(&self, devices: &[AudioDeviceInfo]) -> Result<(), String> {
        self.validate_elevenlabs_outbound_setup()?;
        if self.uses_provider_tts_for_outbound()
            && self.soniox.soniox_tts_outbound_voice.trim().is_empty()
        {
            return Err(
                "Soniox TTS voice is required for You → Meeting Engine voice. Choose a voice in Settings → Voice."
                    .into(),
            );
        }
        validate_for_start_outbound(self, devices)
    }

    pub fn validate_for_start_inbound(&self, devices: &[AudioDeviceInfo]) -> Result<(), String> {
        self.validate_elevenlabs_inbound_setup()?;
        if self.uses_provider_tts_for_inbound() && self.soniox.soniox_tts_voice.trim().is_empty() {
            return Err(
                "Soniox TTS voice is required for Meeting → You translate. Choose a voice in Settings → Voice."
                    .into(),
            );
        }
        validate_for_start_inbound(self, devices)
    }

    pub fn validate_for_direct_outbound(&self, devices: &[AudioDeviceInfo]) -> Result<(), String> {
        validate_for_direct_outbound(self, devices)
    }

    pub fn validate_for_direct_inbound(&self, devices: &[AudioDeviceInfo]) -> Result<(), String> {
        validate_for_direct_inbound(self, devices)
    }
}
