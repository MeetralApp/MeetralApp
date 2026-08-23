use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::audio::{AudioModeHandle, PLAYBACK_PCM_CHANNEL_DEPTH};
use crate::capabilities::{uses_provider_tts_for_outbound, uses_separate_tts};
use crate::config::AppConfig;
use crate::providers::shared::live::{
    BridgeFatalSender, BridgeStatusSender, LiveBridgeHandle, ReconnectPolicy, TranscriptSender,
};
use crate::runtime::factories::live_setup_for_provider;
use crate::runtime::factories::{
    connect_live_bridge_for, spawn_outbound_fanout, FanoutSpawnParams,
};
use crate::runtime::playback_mux::spawn_outbound_playback_mux;
use crate::runtime::voice_runtime::{
    ensure_custom_worker, ensure_provider_tts_worker, spawn_custom_idle_watcher,
    OutboundVoiceRuntime,
};

use super::{OutboundPipeline, OutboundSessionTasks, OutboundStartConnect};

impl OutboundPipeline {
    pub(crate) async fn connect_for_start(
        config: &AppConfig,
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        cancel: CancellationToken,
        pcm_drops: Arc<AtomicU64>,
    ) -> Result<OutboundStartConnect> {
        // Unified topology is the only path (legacy sidecar removed in architecture refactor).
        Self::connect_unified(
            config,
            transcript_tx,
            fatal_tx,
            status_tx,
            cancel,
            pcm_drops,
        )
        .await
    }

    async fn connect_unified(
        config: &AppConfig,
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        cancel: CancellationToken,
        pcm_drops: Arc<AtomicU64>,
    ) -> Result<OutboundStartConnect> {
        let audio_mode = AudioModeHandle::new(config.outbound_mode);
        let shared_mode = audio_mode.shared_mode();
        let playback_generation = audio_mode.shared_generation();

        let el_parent_cancel = cancel.child_token();
        let (custom_pcm_tx, custom_pcm_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (provider_tts_pcm_tx, provider_tts_pcm_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (mux_tx, mux_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let (fanout_tx, fanout_rx) =
            mpsc::channel(crate::runtime::control_channel::TRANSCRIPT_FANOUT_CHANNEL_DEPTH);
        let (latency_tx, latency_rx) =
            mpsc::channel(crate::runtime::control_channel::VOICE_LATENCY_CHANNEL_DEPTH);
        let (voice_tts_status_tx, voice_tts_status_rx) =
            mpsc::channel(crate::runtime::control_channel::TTS_STATUS_CHANNEL_DEPTH);

        let voice_runtime = OutboundVoiceRuntime::new(
            config,
            shared_mode.clone(),
            el_parent_cancel.clone(),
            custom_pcm_tx,
            provider_tts_pcm_tx,
            Some(latency_tx.clone()),
            playback_generation,
            pcm_drops.clone(),
        );

        let (bridge, bridge_pcm_rx) = Self::connect_bridge(
            config,
            fanout_tx,
            fatal_tx,
            status_tx,
            voice_runtime.bridge_play_audio.clone(),
            pcm_drops.clone(),
        )
        .await?;

        let fanout = spawn_outbound_fanout(FanoutSpawnParams {
            cancel: cancel.clone(),
            bridge_rx: fanout_rx,
            engine_tx: transcript_tx,
            tts_cmd_tx: voice_runtime.tts_cmd_tx.clone(),
            audio_mode: shared_mode.clone(),
            voice_engine: voice_runtime.voice_engine.clone(),
            voice_switch_in_progress: voice_runtime.voice_switch_in_progress.clone(),
            relay_chars_while_provider: voice_runtime.relay_chars_while_provider.clone(),
            latency_tx: Some(latency_tx),
            turn_latency: voice_runtime.turn_latency.clone(),
            ai_provider: config.ai_provider,
            voice_output: config.outbound_voice_output,
            synthesis_mode: config.elevenlabs.elevenlabs_tts_synthesis_mode,
            expected_direction: "outbound".to_string(),
        });

        let mux = spawn_outbound_playback_mux(
            cancel.clone(),
            shared_mode.clone(),
            voice_runtime.voice_engine.clone(),
            voice_runtime.mux_generation.clone(),
            uses_separate_tts(config.ai_provider),
            bridge_pcm_rx,
            provider_tts_pcm_rx,
            custom_pcm_rx,
            mux_tx,
            pcm_drops,
        );

        let idle_watcher = spawn_custom_idle_watcher(voice_runtime.clone(), cancel.clone());

        let voice_tts_status_for_connect = if config.needs_custom_tts_for_outbound() {
            ensure_custom_worker(&voice_runtime, config, Some(voice_tts_status_tx))
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            Some(voice_tts_status_rx)
        } else if uses_provider_tts_for_outbound(config) {
            ensure_provider_tts_worker(
                &voice_runtime,
                config,
                &config.meeting_language,
                Some(voice_tts_status_tx),
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
            Some(voice_tts_status_rx)
        } else {
            None
        };

        Ok(OutboundStartConnect {
            bridge,
            playback_rx: mux_rx,
            audio_mode: Some(audio_mode),
            session_tasks: Some(OutboundSessionTasks {
                tts_worker: None,
                fanout,
                mux: Some(mux),
                idle_watcher: Some(idle_watcher),
            }),
            voice_runtime: Some(voice_runtime),
            voice_tts_status_rx: voice_tts_status_for_connect,
            voice_latency_rx: Some(latency_rx),
        })
    }

    pub async fn connect_bridge(
        config: &AppConfig,
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        play_audio: Arc<AtomicBool>,
        pcm_drops: Arc<AtomicU64>,
    ) -> Result<(LiveBridgeHandle, mpsc::Receiver<Vec<i16>>)> {
        let (bridge_audio_tx, bridge_audio_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let setup_options = live_setup_for_provider(config);
        let bridge = connect_live_bridge_for(
            config.ai_provider,
            config.active_api_key(),
            &config.meeting_language,
            "outbound",
            setup_options,
            play_audio,
            bridge_audio_tx,
            pcm_drops,
            transcript_tx,
            fatal_tx,
            status_tx,
            ReconnectPolicy::MeetingGrade,
        )
        .await?;
        Ok((bridge, bridge_audio_rx))
    }
}
