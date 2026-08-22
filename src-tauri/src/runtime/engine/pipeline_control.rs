use super::types::PipelineState;
use super::TranslationEngine;
use crate::audio::list_devices_async;
use crate::config::{AppConfig, PipelineOutputMode};
use tauri::AppHandle;

impl TranslationEngine {
    pub async fn set_outbound_output_mode(
        &mut self,
        mode: PipelineOutputMode,
        config: &AppConfig,
        app: &AppHandle,
    ) -> Result<(), String> {
        if self.status.0 != PipelineState::Active {
            return Err("Outbound pipeline is not active".into());
        }
        if config.session_mode.is_notes() && mode == PipelineOutputMode::Translated {
            return Err(
                "Notes mode only supports original audio (passthrough), not translated TTS".into(),
            );
        }
        let leaving_translated = mode != PipelineOutputMode::Translated;
        let entering_translated = mode == PipelineOutputMode::Translated;
        self.outbound.set_audio_mode(mode)?;
        if let Some(runtime) = self.outbound.voice_runtime() {
            use crate::runtime::voice_runtime::{
                ensure_provider_tts_worker, stop_all_tts_workers, sync_bridge_play_audio,
            };
            if leaving_translated {
                stop_all_tts_workers(&runtime).await;
                sync_bridge_play_audio(&runtime);
            } else if entering_translated {
                if config.uses_provider_tts_for_outbound() {
                    ensure_provider_tts_worker(&runtime, config, &config.meeting_language, None)
                        .await?;
                }
                sync_bridge_play_audio(&runtime);
            }
        }
        self.publish_state(app);
        Ok(())
    }

    pub async fn set_inbound_output_mode(
        &mut self,
        mode: PipelineOutputMode,
        config: &AppConfig,
        app: &AppHandle,
    ) -> Result<(), String> {
        if self.status.1 != PipelineState::Active {
            return Err("Inbound pipeline is not active".into());
        }
        if config.session_mode.is_notes() && mode == PipelineOutputMode::Translated {
            return Err(
                "Notes mode only supports original audio (passthrough), not translated TTS".into(),
            );
        }
        let leaving_translated = mode != PipelineOutputMode::Translated;
        let entering_translated = mode == PipelineOutputMode::Translated;
        if crate::capabilities::scaffolds_inbound_text_tts(config) {
            if leaving_translated {
                // Gate fanout first so no TTS cmds race a closing worker.
                self.inbound.set_audio_mode(mode)?;
                self.inbound.stop_provider_tts().await;
            } else if entering_translated {
                // Start worker before enabling Translated so fanout has a live cmd channel.
                self.inbound.ensure_inbound_translated_tts(config).await?;
                self.inbound.set_audio_mode(mode)?;
                // Captions-first starts with an empty playback target; bind LocalPlayback now.
                let devices = list_devices_async().await.map_err(|e| e.to_string())?;
                self.inbound
                    .restart_playback(config, &devices)
                    .map_err(|e| e.to_string())?;
            } else {
                self.inbound.set_audio_mode(mode)?;
            }
        } else {
            self.inbound.set_audio_mode(mode)?;
        }
        self.publish_state(app);
        Ok(())
    }
}
