use super::app_config::AppConfig;

impl AppConfig {
    pub fn elevenlabs_chunk_schedule(&self) -> [u32; 4] {
        crate::voice::config::chunk_schedule_for_preset(
            self.elevenlabs.elevenlabs_chunk_schedule_preset,
        )
    }

    pub fn resolve_elevenlabs_tts_language_code(&self) -> Option<String> {
        let raw = if self.elevenlabs.elevenlabs_tts_language_auto {
            self.meeting_language.as_str()
        } else {
            self.elevenlabs.elevenlabs_tts_language_code.as_str()
        };
        let code = raw.trim();
        if code.is_empty() {
            None
        } else {
            Some(code.to_lowercase())
        }
    }

    pub fn elevenlabs_init_settings(
        &self,
    ) -> crate::voice::elevenlabs::protocol::ElevenLabsInitSettings {
        let chunk_schedule = match self.elevenlabs.elevenlabs_tts_synthesis_mode {
            crate::voice::config::TtsSynthesisMode::Streaming => {
                Some(self.elevenlabs_chunk_schedule())
            }
            crate::voice::config::TtsSynthesisMode::Sentence => None,
        };
        crate::voice::elevenlabs::protocol::ElevenLabsInitSettings {
            stability: self.elevenlabs.elevenlabs_stability,
            similarity_boost: self.elevenlabs.elevenlabs_similarity_boost,
            speed: self.elevenlabs.elevenlabs_speed,
            use_speaker_boost: self.elevenlabs.elevenlabs_use_speaker_boost,
            chunk_schedule,
        }
    }

    /// Meeting → You EL language follows Translate → You speak (`my_language`).
    pub fn resolve_elevenlabs_inbound_tts_language_code(&self) -> Option<String> {
        let code = self.my_language.trim();
        if code.is_empty() {
            None
        } else {
            Some(code.to_lowercase())
        }
    }

    pub fn elevenlabs_inbound_init_settings(
        &self,
    ) -> crate::voice::elevenlabs::protocol::ElevenLabsInitSettings {
        let chunk_schedule = match self.elevenlabs.elevenlabs_inbound_tts_synthesis_mode {
            crate::voice::config::TtsSynthesisMode::Streaming => {
                Some(self.elevenlabs_chunk_schedule())
            }
            crate::voice::config::TtsSynthesisMode::Sentence => None,
        };
        crate::voice::elevenlabs::protocol::ElevenLabsInitSettings {
            stability: self.elevenlabs.elevenlabs_inbound_stability,
            similarity_boost: self.elevenlabs.elevenlabs_inbound_similarity_boost,
            speed: self.elevenlabs.elevenlabs_speed,
            use_speaker_boost: self.elevenlabs.elevenlabs_use_speaker_boost,
            chunk_schedule,
        }
    }

    /// Provider-native voice uses a separate TTS WebSocket (capability, not vendor name).
    pub fn uses_provider_tts_for_outbound(&self) -> bool {
        crate::capabilities::uses_provider_tts_for_outbound(self)
    }

    pub fn uses_provider_tts_for_inbound(&self) -> bool {
        crate::capabilities::uses_provider_tts_for_inbound(self)
    }
}
