use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::audio::{
    gate_pcm_in_place, resolve_role_device, send_passthrough, shared_playback_device,
    spawn_pipeline_audio, start_user_mic_capture, AudioDeviceInfo, AudioFaultSender,
    AudioModeHandle, AudioRole, CaptureHeartbeat, PlaybackCrossfadeOptions, ResolvedDevice,
    CAPTURE_CHANNEL_DEPTH,
};
use crate::config::AppConfig;
use crate::providers::shared::live::{
    BridgeFatalSender, BridgeStatusSender, LiveBridgeHandle, TranscriptSender,
};

use super::{unix_ms_now, OutboundPipeline, OutboundStartConnect, PASSTHROUGH_DEPTH};

impl OutboundPipeline {
    pub async fn start(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        cancel: CancellationToken,
        mic_muted: Arc<AtomicBool>,
        audio_fault_tx: Option<AudioFaultSender>,
    ) -> Result<()> {
        let connected = Self::connect_for_start(
            config,
            transcript_tx,
            fatal_tx,
            status_tx,
            cancel.clone(),
            Arc::new(std::sync::atomic::AtomicU64::new(0)),
        )
        .await?;

        self.start_with_bridge(
            config,
            devices,
            connected,
            cancel,
            mic_muted,
            audio_fault_tx,
        )
        .await
    }

    pub(crate) async fn start_with_bridge(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        connected: OutboundStartConnect,
        cancel: CancellationToken,
        mic_muted: Arc<AtomicBool>,
        audio_fault_tx: Option<AudioFaultSender>,
    ) -> Result<()> {
        let OutboundStartConnect {
            bridge,
            playback_rx: bridge_audio_rx,
            audio_mode: pre_audio_mode,
            session_tasks,
            voice_runtime,
            voice_tts_status_rx: _,
            voice_latency_rx: _,
        } = connected;

        let audio_mode =
            pre_audio_mode.unwrap_or_else(|| AudioModeHandle::new(config.outbound_mode));
        let (passthrough_tx, passthrough_rx) = mpsc::channel(PASSTHROUGH_DEPTH);
        let bridge_ready = if voice_runtime.is_some() || config.needs_custom_tts_for_outbound() {
            None
        } else {
            Some(bridge.ready_flag())
        };

        let teams_mic_feed = if config.outbound_mode.needs_playback() {
            let resolved = resolve_role_device(AudioRole::TeamsMicFeed, config, devices)
                .map_err(|e| anyhow::anyhow!(e))?;
            tracing::info!("outbound playback → {} ({})", resolved.name, resolved.id);
            resolved
        } else {
            ResolvedDevice {
                id: String::new(),
                name: String::new(),
                direction: "output",
            }
        };
        let playback_device = shared_playback_device(teams_mic_feed);

        let crossfade = if config.elevenlabs.elevenlabs_playback_crossfade {
            Some(PlaybackCrossfadeOptions {
                crossfade_ms: config.elevenlabs.elevenlabs_crossfade_ms,
                flush_aware: true,
            })
        } else {
            None
        };
        let voice_engine = voice_runtime
            .as_ref()
            .map(|runtime| runtime.voice_engine.clone());
        let pcm_drops = voice_runtime
            .as_ref()
            .map(|runtime| runtime.pcm_drops.clone())
            .unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicU64::new(0)));

        // Open mic capture before playback so Bluetooth HFP (16 kHz) settles
        // before the TTS ring buffer latches an A2DP/44.1 kHz nominal rate.
        let (capture_attach_tx, capture_attach_rx) = mpsc::channel::<mpsc::Receiver<Vec<i16>>>(4);
        let (capture_tx, capture_rx) = mpsc::channel(CAPTURE_CHANNEL_DEPTH);
        let heartbeat = CaptureHeartbeat::new();
        let capture = start_user_mic_capture(
            config,
            devices,
            capture_tx,
            heartbeat.clone(),
            audio_fault_tx,
        )?;

        let bridge_for_capture = bridge.clone();
        let cancel_capture = cancel.clone();
        let mic_muted_capture = mic_muted.clone();
        let passthrough_for_capture = passthrough_tx.clone();
        spawn_outbound_capture_forward(
            cancel_capture,
            capture_attach_rx,
            capture_rx,
            passthrough_for_capture,
            bridge_for_capture,
            mic_muted_capture,
            config.record_meeting_audio,
        );

        spawn_pipeline_audio(
            cancel.clone(),
            audio_mode.shared_mode(),
            audio_mode.shared_generation(),
            playback_device.clone(),
            bridge_audio_rx,
            passthrough_rx,
            None,
            bridge_ready,
            crossfade,
            voice_engine,
            None,
            None,
            pcm_drops,
        );

        info!(
            "outbound pipeline started ({} -> {}, mode={:?}, voice={:?}, unified={})",
            config.my_language,
            config.meeting_language,
            config.outbound_mode,
            config.outbound_voice_output,
            voice_runtime.is_some(),
        );

        self.capture_device_id = Some(capture.device_id().to_string());
        self.heartbeat = heartbeat;
        self.started_at_ms = Some(unix_ms_now());
        self.capture = Some(capture);
        self.bridge = Some(bridge);
        self.audio_mode = Some(audio_mode);
        self.session_tasks = session_tasks;
        self.voice_runtime = voice_runtime;
        self.capture_attach_tx = Some(capture_attach_tx);
        self.playback_device = Some(playback_device);

        Ok(())
    }
}

fn spawn_outbound_capture_forward(
    cancel: CancellationToken,
    mut attach_rx: mpsc::Receiver<mpsc::Receiver<Vec<i16>>>,
    initial_rx: mpsc::Receiver<Vec<i16>>,
    passthrough_tx: mpsc::Sender<Vec<i16>>,
    bridge: LiveBridgeHandle,
    mic_muted: Arc<AtomicBool>,
    record_meeting_audio: bool,
) {
    tokio::spawn(async move {
        let mut capture_rx = initial_rx;
        loop {
            if cancel.is_cancelled() {
                break;
            }
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                attached = attach_rx.recv() => {
                    if let Some(rx) = attached {
                        capture_rx = rx;
                    } else {
                        break;
                    }
                }
                pcm = capture_rx.recv() => {
                    match pcm {
                        Some(mut pcm) => {
                            crate::meeting::recording::tap_pcm(
                                record_meeting_audio,
                                "outbound",
                                &pcm,
                            );
                            if !mic_muted.load(std::sync::atomic::Ordering::Relaxed) {
                                bridge.send_audio(&pcm);
                            }
                            gate_pcm_in_place(&mic_muted, &mut pcm);
                            send_passthrough(&passthrough_tx, pcm);
                        }
                        None => {
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }
    });
}
