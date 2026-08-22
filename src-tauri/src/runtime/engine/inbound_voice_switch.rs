use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tracing::info;

use super::types::PipelineState;
use super::TranslationEngine;
use crate::capabilities::{bridge_emits_playback_audio, bridge_play_audio_enabled};
use crate::config::{AppConfig, InboundVoiceOutput, PipelineOutputMode};
use crate::pipeline::inbound::inbound_voice_output_to_engine;
use crate::runtime::voice_runtime::VOICE_ENGINE_CLONE;

const VOICE_SWITCH_COOLDOWN: Duration = Duration::from_millis(800);

pub async fn set_inbound_voice_output(
    engine: &mut TranslationEngine,
    config: &mut AppConfig,
    voice_output: InboundVoiceOutput,
    app: &AppHandle,
) -> Result<(), String> {
    if voice_output.uses_elevenlabs() {
        let mut candidate = config.clone();
        candidate.inbound_voice_output = voice_output;
        candidate.validate_elevenlabs_inbound_setup()?;
    }

    let target_engine = inbound_voice_output_to_engine(voice_output);
    let active = engine.pipeline_states().1 == PipelineState::Active;
    if !active || config.inbound_mode != PipelineOutputMode::Translated {
        config.inbound_voice_output = voice_output;
        engine.publish_state(app);
        return Ok(());
    }

    let (
        current,
        switch_mutex,
        switch_in_progress,
        last_switch_at,
        voice_engine,
        bridge_play_audio,
        mux_generation,
        playback_generation,
    ) = {
        let runtime = engine.inbound.provider_tts_runtime_mut().ok_or_else(|| {
            "Inbound voice switching is not available; restart Meeting Translate".to_string()
        })?;
        (
            runtime.voice_engine.load(Ordering::SeqCst),
            runtime.voice_switch_mutex.clone(),
            runtime.voice_switch_in_progress.clone(),
            runtime.last_switch_at.clone(),
            runtime.voice_engine.clone(),
            runtime.bridge_play_audio.clone(),
            runtime.mux_generation.clone(),
            runtime.playback_generation.clone(),
        )
    };

    if current == target_engine {
        config.inbound_voice_output = voice_output;
        engine.publish_state(app);
        return Ok(());
    }
    if switch_in_progress.load(Ordering::SeqCst) {
        return Err("Voice switch in progress".into());
    }
    let elapsed = last_switch_at
        .lock()
        .map_err(|_| "inbound voice switch clock lock poisoned".to_string())?
        .elapsed();
    if elapsed < VOICE_SWITCH_COOLDOWN {
        return Err(format!(
            "Please wait {}ms before switching again",
            (VOICE_SWITCH_COOLDOWN - elapsed).as_millis()
        ));
    }

    let _guard = switch_mutex.lock().await;
    switch_in_progress.store(true, Ordering::SeqCst);
    bridge_play_audio.store(false, Ordering::SeqCst);
    mux_generation.fetch_add(1, Ordering::SeqCst);
    playback_generation.fetch_add(1, Ordering::SeqCst);

    let mut target_config = config.clone();
    target_config.inbound_voice_output = voice_output;
    let result = if target_engine == VOICE_ENGINE_CLONE {
        engine
            .inbound
            .ensure_inbound_el_worker(&target_config)
            .await
    } else {
        engine.inbound.stop_provider_tts().await;
        engine.inbound.ensure_provider_tts(&target_config).await
    };

    if result.is_ok() {
        voice_engine.store(target_engine, Ordering::SeqCst);
        config.inbound_voice_output = voice_output;
        *last_switch_at
            .lock()
            .map_err(|_| "inbound voice switch clock lock poisoned".to_string())? = Instant::now();
    } else if current == VOICE_ENGINE_CLONE {
        let _ = engine.inbound.ensure_inbound_el_worker(config).await;
    } else {
        let _ = engine.inbound.ensure_provider_tts(config).await;
    }

    let effective_engine = voice_engine.load(Ordering::SeqCst);
    bridge_play_audio.store(
        bridge_play_audio_enabled(
            config.inbound_mode,
            effective_engine,
            bridge_emits_playback_audio(config.ai_provider),
        ),
        Ordering::SeqCst,
    );
    mux_generation.fetch_add(1, Ordering::SeqCst);
    switch_in_progress.store(false, Ordering::SeqCst);

    result?;
    info!(to = ?voice_output, "inbound voice engine switched");
    engine.publish_state(app);
    Ok(())
}
