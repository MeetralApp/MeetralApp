//! Custom-voice factory — ElevenLabs vs Fish Audio spawn/validate only.
//! Pipeline / engine call this; they must not `match` vendor identity.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::audio::pcm_crossfade::PlaybackPcmChunk;
use crate::config::{AppConfig, CustomVoiceVendor};
use crate::providers::fishaudio::{spawn_fishaudio_tts_worker, FishAudioWorkerConfig};
use crate::runtime::control_channel;
use crate::runtime::voice_runtime::OutboundTtsSession;
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::{
    spawn_elevenlabs_tts_worker, ElevenLabsWorkerConfig, TtsTextCommand, VoiceTtsStatus,
};

const WORKER_READY_TIMEOUT: Duration = Duration::from_secs(15);
const WORKER_JOIN_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomVoiceDirection {
    Outbound,
    Inbound,
}

fn vendor_for(config: &AppConfig, direction: CustomVoiceDirection) -> CustomVoiceVendor {
    match direction {
        CustomVoiceDirection::Outbound => config.outbound_custom_voice_vendor,
        CustomVoiceDirection::Inbound => config.inbound_custom_voice_vendor,
    }
}

fn vendor_label(vendor: CustomVoiceVendor) -> &'static str {
    match vendor {
        CustomVoiceVendor::ElevenLabs => "ElevenLabs",
        CustomVoiceVendor::FishAudio => "Fish Audio",
    }
}

/// REST check that the outbound custom voice still exists (start-outbound gate).
pub async fn validate_outbound_custom_voice(config: &AppConfig) -> Result<(), String> {
    match config.outbound_custom_voice_vendor {
        CustomVoiceVendor::ElevenLabs => {
            crate::voice::validate_elevenlabs_voice(
                &config.elevenlabs.elevenlabs_api_key,
                &config.elevenlabs.elevenlabs_voice_id,
            )
            .await
        }
        CustomVoiceVendor::FishAudio => {
            crate::providers::fishaudio::validate_voice(
                &config.fishaudio.fishaudio_api_key,
                &config.fishaudio.fishaudio_voice_id,
            )
            .await
        }
    }
}

/// Spawn the custom-voice WebSocket for one direction. Vendor match lives here only.
pub async fn spawn_custom_tts_session(
    config: &AppConfig,
    direction: CustomVoiceDirection,
    tts_cmd_rx: mpsc::Receiver<TtsTextCommand>,
    pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    turn_latency: Arc<TurnLatencySlot>,
    parent_cancel: CancellationToken,
    app_status_tx: Option<mpsc::Sender<VoiceTtsStatus>>,
) -> Result<OutboundTtsSession, String> {
    let vendor = vendor_for(config, direction);
    let label = vendor_label(vendor);
    let (worker_status_tx, worker_status_rx) =
        mpsc::channel(control_channel::TTS_STATUS_CHANNEL_DEPTH);
    let worker_cancel = parent_cancel.child_token();
    let forward_cancel = worker_cancel.child_token();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let mut ready_tx = Some(ready_tx);

    tokio::spawn(async move {
        let mut rx = worker_status_rx;
        let mut ready_signaled = false;
        while let Some(status) = rx.recv().await {
            let is_ready = matches!(status, VoiceTtsStatus::Ready);
            let degraded = if let VoiceTtsStatus::Degraded { ref message } = status {
                Some(message.clone())
            } else {
                None
            };
            if let Some(ref app_tx) = app_status_tx {
                control_channel::try_send_control(app_tx, status, "tts-status");
            }
            if is_ready && !ready_signaled {
                ready_signaled = true;
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(Ok(()));
                }
            } else if let Some(msg) = degraded {
                if !ready_signaled {
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(Err(msg));
                    }
                    return;
                }
            }
        }
        if !ready_signaled {
            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(Err(format!("{label} worker stopped unexpectedly")));
            }
        }
    });

    let worker = match vendor {
        CustomVoiceVendor::ElevenLabs => spawn_elevenlabs_tts_worker(
            elevenlabs_worker_config(config, direction),
            tts_cmd_rx,
            pcm_tx,
            pcm_drops,
            worker_status_tx,
            turn_latency,
            worker_cancel.clone(),
        ),
        CustomVoiceVendor::FishAudio => spawn_fishaudio_tts_worker(
            FishAudioWorkerConfig {
                api_key: config.fishaudio.fishaudio_api_key.clone(),
                init_settings: match direction {
                    CustomVoiceDirection::Outbound => config.fishaudio_outbound_init_settings(),
                    CustomVoiceDirection::Inbound => config.fishaudio_inbound_init_settings(),
                },
            },
            tts_cmd_rx,
            pcm_tx,
            pcm_drops,
            worker_status_tx,
            turn_latency,
            worker_cancel.clone(),
        ),
    };

    match tokio::time::timeout(WORKER_READY_TIMEOUT, ready_rx).await {
        Ok(Ok(Ok(()))) => Ok(OutboundTtsSession {
            worker,
            worker_cancel,
            forward_cancel,
        }),
        Ok(Ok(Err(message))) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(WORKER_JOIN_TIMEOUT, worker).await;
            Err(message)
        }
        Ok(Err(_)) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(WORKER_JOIN_TIMEOUT, worker).await;
            Err(format!("{label} worker stopped unexpectedly"))
        }
        Err(_) => {
            worker_cancel.cancel();
            forward_cancel.cancel();
            let _ = tokio::time::timeout(WORKER_JOIN_TIMEOUT, worker).await;
            Err(format!("{label} connect timeout (15s)"))
        }
    }
}

fn elevenlabs_worker_config(
    config: &AppConfig,
    direction: CustomVoiceDirection,
) -> ElevenLabsWorkerConfig {
    match direction {
        CustomVoiceDirection::Outbound => ElevenLabsWorkerConfig {
            api_key: config.elevenlabs.elevenlabs_api_key.clone(),
            voice_id: config.elevenlabs.elevenlabs_voice_id.clone(),
            model_id: config.elevenlabs.elevenlabs_tts_model.clone(),
            init_settings: config.elevenlabs_init_settings(),
            language_code: config.resolve_elevenlabs_tts_language_code(),
            auto_mode: config.elevenlabs.elevenlabs_auto_mode,
        },
        CustomVoiceDirection::Inbound => ElevenLabsWorkerConfig {
            api_key: config.elevenlabs.elevenlabs_api_key.clone(),
            voice_id: config.elevenlabs.elevenlabs_inbound_voice_id.clone(),
            model_id: config.elevenlabs.elevenlabs_inbound_tts_model.clone(),
            init_settings: config.elevenlabs_inbound_init_settings(),
            language_code: config.resolve_elevenlabs_inbound_tts_language_code(),
            auto_mode: config.elevenlabs.elevenlabs_auto_mode,
        },
    }
}
