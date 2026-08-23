use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8},
    Arc, Mutex as StdMutex,
};
use std::time::Instant;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::audio::pcm_crossfade::PlaybackPcmChunk;
use crate::audio::{AudioModeHandle, PLAYBACK_PCM_CHANNEL_DEPTH};
use crate::capabilities::{
    bridge_emits_playback_audio, bridge_play_audio_enabled, needs_custom_tts_for_inbound,
    scaffolds_inbound_text_tts, uses_provider_tts_for_inbound, uses_separate_tts,
};
use crate::config::{AppConfig, InboundVoiceOutput};
use crate::providers::shared::live::{
    BridgeFatalSender, BridgeStatusSender, ReconnectPolicy, TranscriptSender,
};
use crate::runtime::factories::{
    connect_live_bridge_for, live_setup_for_provider, spawn_custom_tts_session,
    spawn_inbound_fanout, CustomVoiceDirection, InboundFanoutSpawnParams,
};
use crate::runtime::playback_mux::spawn_outbound_playback_mux;
use crate::runtime::voice_runtime::{VOICE_ENGINE_CUSTOM, VOICE_ENGINE_PROVIDER};
use crate::voice::elevenlabs::latency::TurnLatencySlot;

use super::provider_tts::spawn_inbound_provider_tts_session;
use super::{InboundBridgeConnect, InboundPipeline, InboundProviderTts};

pub(crate) fn inbound_voice_output_to_engine(output: InboundVoiceOutput) -> u8 {
    match output {
        InboundVoiceOutput::ProviderNative => VOICE_ENGINE_PROVIDER,
        InboundVoiceOutput::Custom => VOICE_ENGINE_CUSTOM,
    }
}

impl InboundPipeline {
    pub async fn connect_bridge(
        config: &AppConfig,
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        cancel: CancellationToken,
        pcm_drops: Arc<AtomicU64>,
    ) -> Result<InboundBridgeConnect> {
        let (bridge_audio_tx, bridge_audio_rx) = mpsc::channel(PLAYBACK_PCM_CHANNEL_DEPTH);
        let setup_options = live_setup_for_provider(config);
        let initial_engine = inbound_voice_output_to_engine(config.inbound_voice_output);
        let play_audio = Arc::new(AtomicBool::new(bridge_play_audio_enabled(
            config.inbound_mode,
            initial_engine,
            bridge_emits_playback_audio(config.ai_provider),
        )));

        let scaffold_text_tts = scaffolds_inbound_text_tts(config);
        let (bridge_transcript_tx, bridge_transcript_rx) = if scaffold_text_tts {
            let (tx, rx) =
                mpsc::channel(crate::runtime::control_channel::TRANSCRIPT_FANOUT_CHANNEL_DEPTH);
            (tx, Some(rx))
        } else {
            (transcript_tx.clone(), None)
        };

        let bridge = connect_live_bridge_for(
            config.ai_provider,
            config.active_api_key(),
            &config.my_language,
            "inbound",
            setup_options,
            play_audio.clone(),
            bridge_audio_tx,
            pcm_drops.clone(),
            bridge_transcript_tx,
            fatal_tx,
            status_tx,
            ReconnectPolicy::MeetingGrade,
        )
        .await?;

        if let Some(bridge_te_rx) = bridge_transcript_rx {
            let (tts_cmd_tx, tts_cmd_rx) =
                mpsc::channel(crate::runtime::control_channel::TTS_CMD_CHANNEL_DEPTH);
            drop(tts_cmd_rx);
            let (provider_tts_pcm_tx, provider_tts_pcm_rx) =
                mpsc::channel::<PlaybackPcmChunk>(PLAYBACK_PCM_CHANNEL_DEPTH);
            let (custom_pcm_tx, custom_pcm_rx) =
                mpsc::channel::<PlaybackPcmChunk>(PLAYBACK_PCM_CHANNEL_DEPTH);
            let (mux_tx, mux_rx) = mpsc::channel::<PlaybackPcmChunk>(PLAYBACK_PCM_CHANNEL_DEPTH);

            let audio_mode = AudioModeHandle::new(config.inbound_mode);
            let shared_mode = audio_mode.shared_mode();
            let playback_generation = audio_mode.shared_generation();
            let voice_engine = Arc::new(AtomicU8::new(initial_engine));
            let switch_flag = Arc::new(AtomicBool::new(false));
            let relay_chars = Arc::new(AtomicU64::new(0));
            let tts_cmd_shared = Arc::new(StdMutex::new(tts_cmd_tx));
            let turn_latency = TurnLatencySlot::new_shared();
            let mux_generation = Arc::new(AtomicU64::new(0));

            spawn_inbound_fanout(InboundFanoutSpawnParams {
                cancel: cancel.clone(),
                bridge_rx: bridge_te_rx,
                engine_tx: transcript_tx,
                tts_cmd_tx: tts_cmd_shared.clone(),
                audio_mode: shared_mode.clone(),
                voice_engine: voice_engine.clone(),
                voice_switch_in_progress: switch_flag.clone(),
                relay_chars_while_provider: relay_chars,
                latency_tx: None,
                turn_latency: turn_latency.clone(),
                ai_provider: config.ai_provider,
                voice_output: config.inbound_voice_output,
                synthesis_mode: config.elevenlabs.elevenlabs_inbound_tts_synthesis_mode,
                expected_direction: "inbound".to_string(),
            });

            spawn_outbound_playback_mux(
                cancel.clone(),
                shared_mode,
                voice_engine.clone(),
                mux_generation.clone(),
                uses_separate_tts(config.ai_provider),
                bridge_audio_rx,
                provider_tts_pcm_rx,
                custom_pcm_rx,
                mux_tx,
                pcm_drops.clone(),
            );

            let mut tts = InboundProviderTts {
                tts_cmd_tx: tts_cmd_shared,
                pcm_tx: provider_tts_pcm_tx.clone(),
                provider_session: None,
                custom_session: None,
                pipeline_cancel: cancel.clone(),
                voice_engine,
                bridge_play_audio: play_audio,
                mux_generation,
                voice_switch_in_progress: switch_flag,
                voice_switch_mutex: Arc::new(tokio::sync::Mutex::new(())),
                last_switch_at: Arc::new(StdMutex::new(Instant::now())),
                provider_tts_pcm_tx,
                custom_pcm_tx,
                playback_generation,
                turn_latency,
                pcm_drops: pcm_drops.clone(),
            };

            if needs_custom_tts_for_inbound(config) {
                let (cmd_tx, cmd_rx) =
                    mpsc::channel(crate::runtime::control_channel::TTS_CMD_CHANNEL_DEPTH);
                *tts.tts_cmd_tx
                    .lock()
                    .map_err(|_| anyhow::anyhow!("inbound tts cmd lock poisoned"))? = cmd_tx;
                tts.custom_session = Some(
                    spawn_custom_tts_session(
                        config,
                        CustomVoiceDirection::Inbound,
                        cmd_rx,
                        tts.custom_pcm_tx.clone(),
                        tts.pcm_drops.clone(),
                        tts.turn_latency.clone(),
                        cancel.clone(),
                        None,
                    )
                    .await
                    .map_err(|e| {
                        cancel.cancel();
                        anyhow::anyhow!(e)
                    })?,
                );
            } else if uses_provider_tts_for_inbound(config) {
                let (cmd_tx, cmd_rx) =
                    mpsc::channel(crate::runtime::control_channel::TTS_CMD_CHANNEL_DEPTH);
                *tts.tts_cmd_tx
                    .lock()
                    .map_err(|_| anyhow::anyhow!("inbound tts cmd lock poisoned"))? = cmd_tx;
                tts.provider_session = Some(
                    spawn_inbound_provider_tts_session(
                        config,
                        cmd_rx,
                        tts.provider_tts_pcm_tx.clone(),
                        tts.pcm_drops.clone(),
                        tts.turn_latency.clone(),
                        cancel.clone(),
                    )
                    .await
                    .map_err(|e| {
                        cancel.cancel();
                        anyhow::anyhow!(e)
                    })?,
                );
            }

            let (_unused_tx, unused_rx) = mpsc::channel::<Vec<i16>>(PLAYBACK_PCM_CHANNEL_DEPTH);
            drop(_unused_tx);

            return Ok(InboundBridgeConnect {
                bridge,
                playback_rx: unused_rx,
                playback_chunks: Some(mux_rx),
                audio_mode: Some(audio_mode),
                provider_tts: Some(tts),
                pcm_drops,
            });
        }

        Ok(InboundBridgeConnect {
            bridge,
            playback_rx: bridge_audio_rx,
            playback_chunks: None,
            audio_mode: None,
            provider_tts: None,
            pcm_drops,
        })
    }
}
