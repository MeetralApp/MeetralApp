use crate::ai::{migrate_config_for_provider, AiProvider};
use crate::providers::shared::live::{LiveSetupOptions, SonioxTranslationTerm};

use super::app_config::AppConfig;
use super::modes::VadSensitivity;

impl AppConfig {
    pub fn migrate_for_provider_switch(&mut self, new_provider: AiProvider) -> bool {
        if self.ai_provider == new_provider {
            return false;
        }
        self.ai_provider = new_provider;
        let mut summary_scratch = self.summary_model.clone();
        let changed = migrate_config_for_provider(
            new_provider,
            &mut self.live_model,
            &mut summary_scratch,
            &mut self.my_language,
            &mut self.meeting_language,
        );
        // summary_provider / summary_model stay independent of the live engine.
        self.normalize();
        changed
    }

    pub fn setup_options(&self) -> LiveSetupOptions {
        let mut hints = Vec::new();
        if !self.my_language.trim().is_empty() {
            hints.push(self.my_language.clone());
        }
        if !self.meeting_language.trim().is_empty() && self.meeting_language != self.my_language {
            hints.push(self.meeting_language.clone());
        }
        let resolved = crate::providers::soniox::resolve_active_soniox_context(
            &self.soniox.soniox_always_on,
            &self.soniox.soniox_context_profiles,
            self.soniox.soniox_active_context_profile_id.as_deref(),
        );
        let translation_terms: Vec<SonioxTranslationTerm> = resolved
            .translation_terms
            .into_iter()
            .map(|(source, target)| SonioxTranslationTerm { source, target })
            .collect();
        LiveSetupOptions {
            model: self.live_model.clone(),
            echo_target_language: self.echo_target_language,
            vad_silence_duration_ms: self.vad_silence_duration_ms,
            vad_start_sensitivity: self.vad_start_sensitivity.to_api_value().to_string(),
            vad_end_sensitivity: self.vad_end_sensitivity.to_end_api_value().to_string(),
            language_hints: hints,
            soniox_general: resolved.general,
            soniox_context_text: resolved.text,
            soniox_glossary_terms: resolved.terms,
            soniox_translation_terms: translation_terms,
            soniox_endpoint_latency_adjustment_level: self
                .soniox
                .soniox_endpoint_latency_adjustment_level,
            soniox_endpoint_sensitivity: self.soniox.soniox_endpoint_sensitivity,
            soniox_max_endpoint_delay_ms: self.soniox.soniox_max_endpoint_delay_ms,
            translation_enabled: self.session_mode.translation_enabled(),
        }
    }

    /// Clone-specific VAD tuning for outbound. **Not** applied on initial Gemini WebSocket
    /// setup — Gemini closes the session when `silenceDurationMs` / end sensitivity differ
    /// from the app defaults on cold connect. Hot-switch Provider→Clone reuses the live
    /// bridge (same setup). Use [`setup_options`] for [`OutboundPipeline::connect_bridge`].
    pub fn outbound_setup_options(&self) -> LiveSetupOptions {
        let mut opts = self.setup_options();
        if self.needs_elevenlabs_for_outbound() {
            opts.vad_silence_duration_ms = 400;
            opts.vad_end_sensitivity = VadSensitivity::Medium.to_end_api_value().to_string();
        }
        opts
    }
}
