use std::sync::{atomic::AtomicBool, Arc, Mutex as StdMutex};

use anyhow::Result;
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::audio::ducking_mix::DuckingParams;
use crate::audio::{
    gate_pcm_in_place, resolve_role_device, send_passthrough, shared_playback_device,
    spawn_pcm24k_adapter, spawn_pipeline_audio, start_meeting_capture_for_config, AudioDeviceInfo,
    AudioFaultSender, AudioModeHandle, AudioRole, CaptureHeartbeat, ResolvedDevice,
    CAPTURE_CHANNEL_DEPTH,
};
use crate::config::AppConfig;
use crate::providers::shared::live::{
    BridgeFatalSender, BridgeStatusSender, LiveBridgeHandle, TranscriptSender,
};

use super::{unix_ms_now, InboundBridgeConnect, InboundPipeline, PASSTHROUGH_DEPTH};

impl InboundPipeline {
    pub async fn start(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        cancel: CancellationToken,
        speaker_muted: Arc<AtomicBool>,
        audio_fault_tx: Option<AudioFaultSender>,
        app: AppHandle,
    ) -> Result<()> {
        let connected = Self::connect_bridge(
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
            speaker_muted,
            audio_fault_tx,
            app,
        )
        .await
    }

    pub async fn start_with_bridge(
        &mut self,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        connected: InboundBridgeConnect,
        cancel: CancellationToken,
        speaker_muted: Arc<AtomicBool>,
        audio_fault_tx: Option<AudioFaultSender>,
        app: AppHandle,
    ) -> Result<()> {
        let InboundBridgeConnect {
            bridge,
            playback_rx: bridge_audio_rx,
            playback_chunks,
            audio_mode: soniox_audio_mode,
            provider_tts,
            pcm_drops,
        } = connected;

        let meeting_capture = resolve_role_device(AudioRole::MeetingCapture, config, devices)
            .map_err(|e| anyhow::anyhow!(e))?;
        tracing::info!(
            "inbound meeting capture → {} ({})",
            meeting_capture.name,
            meeting_capture.id
        );

        let audio_mode =
            soniox_audio_mode.unwrap_or_else(|| AudioModeHandle::new(config.inbound_mode));
        let (passthrough_tx, passthrough_rx) = mpsc::channel(PASSTHROUGH_DEPTH);
        let bridge_ready = bridge.ready_flag();

        let ducking_params = Arc::new(StdMutex::new(DuckingParams::from_config_fields(
            config.inbound_original_under_translation,
            config.inbound_original_ducked_gain,
        )));

        let local_playback = if config.inbound_mode.needs_playback() {
            let resolved = resolve_role_device(AudioRole::LocalPlayback, config, devices)
                .map_err(|e| anyhow::anyhow!(e))?;
            tracing::info!("inbound playback → {} ({})", resolved.name, resolved.id);
            resolved
        } else {
            ResolvedDevice {
                id: String::new(),
                name: String::new(),
                direction: "output",
            }
        };
        let playback_device = shared_playback_device(local_playback);

        let playback_chunks = match playback_chunks {
            Some(rx) => rx,
            None => spawn_pcm24k_adapter(cancel.clone(), bridge_audio_rx, pcm_drops.clone()),
        };

        spawn_pipeline_audio(
            cancel.clone(),
            audio_mode.shared_mode(),
            audio_mode.shared_generation(),
            playback_device.clone(),
            playback_chunks,
            passthrough_rx,
            Some(speaker_muted.clone()),
            Some(bridge_ready),
            None,
            Some(ducking_params.clone()),
            Some(app.clone()),
            pcm_drops,
        );

        let (capture_attach_tx, capture_attach_rx) = mpsc::channel::<mpsc::Receiver<Vec<i16>>>(4);
        let (capture_tx, capture_rx) = mpsc::channel(CAPTURE_CHANNEL_DEPTH);
        let heartbeat = CaptureHeartbeat::new();
        let capture = start_meeting_capture_for_config(
            config,
            devices,
            capture_tx,
            heartbeat.clone(),
            audio_fault_tx,
        )?;

        let bridge_for_capture = bridge.clone();
        let cancel_capture = cancel.clone();
        let speaker_muted_capture = speaker_muted.clone();
        let passthrough_for_capture = passthrough_tx.clone();
        spawn_inbound_capture_forward(
            cancel_capture,
            capture_attach_rx,
            capture_rx,
            passthrough_for_capture,
            bridge_for_capture,
            speaker_muted_capture,
            config.record_meeting_audio,
        );

        info!(
            "inbound pipeline started ({} -> {}, mode={:?})",
            config.meeting_language, config.my_language, config.inbound_mode
        );

        self.capture_device_id = Some(capture.device_id().to_string());
        self.heartbeat = heartbeat;
        self.started_at_ms = Some(unix_ms_now());
        self.capture = Some(capture);
        self.bridge = Some(bridge);
        self.audio_mode = Some(audio_mode);
        self.capture_attach_tx = Some(capture_attach_tx);
        self.playback_device = Some(playback_device);
        self.provider_tts = provider_tts;
        self.ducking_params = Some(ducking_params);

        Ok(())
    }
}

fn spawn_inbound_capture_forward(
    cancel: CancellationToken,
    mut attach_rx: mpsc::Receiver<mpsc::Receiver<Vec<i16>>>,
    initial_rx: mpsc::Receiver<Vec<i16>>,
    passthrough_tx: mpsc::Sender<Vec<i16>>,
    bridge: LiveBridgeHandle,
    speaker_muted: Arc<AtomicBool>,
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
                                "inbound",
                                &pcm,
                            );
                            if !speaker_muted.load(std::sync::atomic::Ordering::Relaxed) {
                                bridge.send_audio(&pcm);
                            }
                            gate_pcm_in_place(&speaker_muted, &mut pcm);
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
