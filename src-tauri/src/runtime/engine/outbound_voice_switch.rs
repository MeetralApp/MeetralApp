use std::sync::Arc;
use std::time::Instant;

use tauri::AppHandle;
use tracing::info;

use super::types::PipelineState;
use super::TranslationEngine;
use crate::config::{AppConfig, OutboundVoiceOutput, PipelineOutputMode};
use crate::runtime::voice_runtime::{
    engine_to_voice_output, ensure_custom_worker, ensure_provider_tts_worker, stop_custom_worker,
    stop_provider_tts_worker, sync_bridge_play_audio, voice_output_to_engine, VOICE_ENGINE_CUSTOM,
    VOICE_ENGINE_PROVIDER,
};

pub async fn set_outbound_voice_output(
    engine: &mut TranslationEngine,
    config: &mut AppConfig,
    voice_output: OutboundVoiceOutput,
    app: &AppHandle,
) -> Result<(), String> {
    if voice_output.uses_custom_tts() {
        config.validate_custom_voice_outbound_setup()?;
    }

    let target_engine = voice_output_to_engine(voice_output);
    let active = engine.pipeline_states().0 == PipelineState::Active;

    if !active || config.outbound_mode != PipelineOutputMode::Translated {
        config.outbound_voice_output = voice_output;
        engine.publish_state(app);
        return Ok(());
    }

    let runtime = engine
        .outbound_voice_runtime()
        .ok_or_else(|| "Voice runtime not available (legacy topology)".to_string())?;

    if runtime
        .voice_switch_in_progress
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Err("Voice switch in progress".into());
    }

    if let Some(remaining) = runtime.cooldown_remaining() {
        return Err(format!(
            "Please wait {}ms before switching again",
            remaining.as_millis()
        ));
    }

    let current = runtime
        .voice_engine
        .load(std::sync::atomic::Ordering::SeqCst);
    if current == target_engine {
        config.outbound_voice_output = voice_output;
        engine.publish_state(app);
        return Ok(());
    }

    let _guard = runtime.voice_switch_mutex.lock().await;
    let switch_start = Instant::now();

    runtime
        .voice_switch_in_progress
        .store(true, std::sync::atomic::Ordering::SeqCst);

    let result = if target_engine == VOICE_ENGINE_CUSTOM {
        switch_to_custom(config, voice_output, &runtime).await
    } else {
        switch_to_provider(config, voice_output, &runtime).await
    };

    runtime
        .voice_switch_in_progress
        .store(false, std::sync::atomic::Ordering::SeqCst);

    match result {
        Ok(()) => {
            *crate::meeting::lock_poison_recover(&runtime.last_switch_at, "voice switch clock") =
                Instant::now();
            let ms = switch_start.elapsed().as_millis();
            let from = engine_to_voice_output(current);
            info!(
                from = ?from,
                to = ?voice_output,
                duration_ms = ms,
                "outbound voice engine switched"
            );
            let leaked = runtime
                .relay_chars_while_provider
                .swap(0, std::sync::atomic::Ordering::Relaxed);
            if leaked > 0 {
                tracing::warn!(
                    chars = leaked,
                    "elevenlabs text sent while provider engine was active (G-COST-5 leak)"
                );
            }
            engine.publish_state(app);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("voice switch failed: {e}");
            Err(e)
        }
    }
}

async fn switch_to_custom(
    config: &mut AppConfig,
    voice_output: OutboundVoiceOutput,
    runtime: &Arc<crate::runtime::voice_runtime::OutboundVoiceRuntime>,
) -> Result<(), String> {
    runtime
        .bridge_play_audio
        .store(false, std::sync::atomic::Ordering::SeqCst);
    runtime.bump_mux_generation();
    runtime.flush_playback();

    match ensure_custom_worker(runtime, config, None).await {
        Ok(()) => {
            runtime
                .voice_engine
                .store(VOICE_ENGINE_CUSTOM, std::sync::atomic::Ordering::SeqCst);
            sync_bridge_play_audio(runtime);
            runtime.bump_mux_generation();
            config.outbound_voice_output = voice_output;
            Ok(())
        }
        Err(e) => {
            runtime
                .voice_engine
                .store(VOICE_ENGINE_PROVIDER, std::sync::atomic::Ordering::SeqCst);
            sync_bridge_play_audio(runtime);
            runtime.bump_mux_generation();
            config.outbound_voice_output = OutboundVoiceOutput::ProviderNative;
            Err(e)
        }
    }
}

async fn switch_to_provider(
    config: &mut AppConfig,
    voice_output: OutboundVoiceOutput,
    runtime: &Arc<crate::runtime::voice_runtime::OutboundVoiceRuntime>,
) -> Result<(), String> {
    runtime
        .voice_engine
        .store(VOICE_ENGINE_PROVIDER, std::sync::atomic::Ordering::SeqCst);
    sync_bridge_play_audio(runtime);

    stop_custom_worker(runtime).await;
    stop_provider_tts_worker(runtime).await;
    runtime.bump_mux_generation();
    runtime.flush_playback();

    if crate::capabilities::uses_separate_tts(config.ai_provider) {
        ensure_provider_tts_worker(runtime, config, &config.meeting_language, None).await?;
    }

    sync_bridge_play_audio(runtime);
    runtime.bump_mux_generation();

    config.outbound_voice_output = voice_output;
    Ok(())
}
