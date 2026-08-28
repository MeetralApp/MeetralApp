use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::audio::PlaybackPcmChunk;
use crate::capabilities::{
    needs_custom_tts_for_inbound, scaffolds_inbound_text_tts, uses_provider_tts_for_inbound,
};
use crate::config::AppConfig;
use crate::runtime::control_channel;
use crate::runtime::factories::{spawn_custom_tts_session, CustomVoiceDirection};
use crate::runtime::voice_runtime::{stop_tts_session, OutboundTtsSession};
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::{
    spawn_soniox_tts_worker, SonioxTtsWorkerConfig, TtsTextCommand, VoiceTtsStatus,
};

use super::InboundPipeline;

const WORKER_READY_TIMEOUT: Duration = Duration::from_secs(15);
const WORKER_JOIN_TIMEOUT: Duration = Duration::from_secs(3);

impl InboundPipeline {
    /// Close all inbound TTS WebSockets. The STT bridge stays up.
    pub async fn stop_provider_tts(&mut self) {
        let Some(tts) = self.provider_tts.as_mut() else {
            return;
        };
        if let Ok(tx) = tts.tts_cmd_tx.lock() {
            control_channel::try_send_control(&tx, TtsTextCommand::Flush, "tts-cmd");
            control_channel::try_send_control(&tx, TtsTextCommand::Reset, "tts-cmd");
        }
        if let Some(session) = tts.provider_session.take() {
            stop_tts_session(session, "inbound provider tts").await;
        }
        if let Some(session) = tts.custom_session.take() {
            stop_tts_session(session, "inbound custom tts").await;
        }
    }

    pub async fn stop_inbound_custom_worker(&mut self) {
        let Some(tts) = self.provider_tts.as_mut() else {
            return;
        };
        if let Ok(tx) = tts.tts_cmd_tx.lock() {
            control_channel::try_send_control(&tx, TtsTextCommand::Flush, "tts-cmd");
            control_channel::try_send_control(&tx, TtsTextCommand::Reset, "tts-cmd");
        }
        if let Some(session) = tts.custom_session.take() {
            stop_tts_session(session, "inbound custom tts").await;
        }
    }

    pub async fn ensure_provider_tts(&mut self, config: &AppConfig) -> Result<(), String> {
        if !uses_provider_tts_for_inbound(config) {
            return Ok(());
        }
        self.stop_inbound_custom_worker().await;
        let Some(tts) = self.provider_tts.as_mut() else {
            return Err("Inbound provider TTS is not available; restart Meeting Translate".into());
        };
        if tts.provider_session.is_some() {
            return Ok(());
        }

        let (tts_cmd_tx, tts_cmd_rx) = mpsc::channel(control_channel::TTS_CMD_CHANNEL_DEPTH);
        {
            let mut guard = tts
                .tts_cmd_tx
                .lock()
                .map_err(|_| "inbound soniox tts cmd lock poisoned".to_string())?;
            *guard = tts_cmd_tx;
        }

        let session = spawn_inbound_provider_tts_session(
            config,
            tts_cmd_rx,
            tts.provider_tts_pcm_tx.clone(),
            tts.pcm_drops.clone(),
            tts.turn_latency.clone(),
            tts.pipeline_cancel.clone(),
        )
        .await?;
        tts.provider_session = Some(session);
        info!("inbound soniox tts worker ready (hot restart)");
        Ok(())
    }

    pub async fn ensure_inbound_custom_worker(&mut self, config: &AppConfig) -> Result<(), String> {
        if !needs_custom_tts_for_inbound(config) {
            return Ok(());
        }
        config.validate_custom_voice_inbound_setup()?;

        if let Some(tts) = self.provider_tts.as_mut() {
            if let Ok(tx) = tts.tts_cmd_tx.lock() {
                control_channel::try_send_control(&tx, TtsTextCommand::Flush, "tts-cmd");
                control_channel::try_send_control(&tx, TtsTextCommand::Reset, "tts-cmd");
            }
            if let Some(session) = tts.provider_session.take() {
                stop_tts_session(session, "inbound provider tts").await;
            }
        }

        let Some(tts) = self.provider_tts.as_mut() else {
            return Err("Inbound custom voice is not available; restart Meeting Translate".into());
        };
        if tts.custom_session.is_some() {
            return Ok(());
        }

        let (tts_cmd_tx, tts_cmd_rx) = mpsc::channel(control_channel::TTS_CMD_CHANNEL_DEPTH);
        {
            let mut guard = tts
                .tts_cmd_tx
                .lock()
                .map_err(|_| "inbound custom tts cmd lock poisoned".to_string())?;
            *guard = tts_cmd_tx;
        }

        let session = spawn_custom_tts_session(
            config,
            CustomVoiceDirection::Inbound,
            tts_cmd_rx,
            tts.custom_pcm_tx.clone(),
            tts.pcm_drops.clone(),
            tts.turn_latency.clone(),
            tts.pipeline_cancel.clone(),
            None,
        )
        .await?;
        tts.custom_session = Some(session);
        info!(
            vendor = config.inbound_custom_voice_vendor.as_log_label(),
            "inbound custom worker ready"
        );
        Ok(())
    }

    pub async fn ensure_inbound_translated_tts(
        &mut self,
        config: &AppConfig,
    ) -> Result<(), String> {
        if !scaffolds_inbound_text_tts(config) {
            return Ok(());
        }
        if needs_custom_tts_for_inbound(config) {
            self.ensure_inbound_custom_worker(config).await
        } else {
            self.ensure_provider_tts(config).await
        }
    }
}

pub(super) async fn spawn_inbound_provider_tts_session(
    config: &AppConfig,
    tts_cmd_rx: mpsc::Receiver<TtsTextCommand>,
    tts_pcm_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: std::sync::Arc<std::sync::atomic::AtomicU64>,
    turn_latency: std::sync::Arc<TurnLatencySlot>,
    pipeline_cancel: CancellationToken,
) -> Result<OutboundTtsSession, String> {
    let (worker_status_tx, mut worker_status_rx) =
        mpsc::channel(control_channel::TTS_STATUS_CHANNEL_DEPTH);
    let worker_cancel = pipeline_cancel.child_token();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let mut ready_tx = Some(ready_tx);
    tokio::spawn(async move {
        let mut ready_signaled = false;
        while let Some(status) = worker_status_rx.recv().await {
            let is_ready = matches!(status, VoiceTtsStatus::Ready);
            let degraded = if let VoiceTtsStatus::Degraded { ref message } = status {
                Some(message.clone())
            } else {
                None
            };
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
                let _ = tx.send(Err("Soniox TTS worker stopped unexpectedly".to_string()));
            }
        }
    });

    let worker = spawn_soniox_tts_worker(
        SonioxTtsWorkerConfig {
            api_key: config.soniox_api_key.clone(),
            voice: config.soniox.soniox_tts_voice.clone(),
            model: config.soniox.soniox_tts_inbound_model.clone(),
            language: config.resolve_soniox_tts_language().to_string(),
            speed: config.soniox.soniox_tts_inbound_speed,
        },
        tts_cmd_rx,
        tts_pcm_tx,
        pcm_drops,
        worker_status_tx,
        turn_latency,
        worker_cancel.clone(),
    );

    match tokio::time::timeout(WORKER_READY_TIMEOUT, ready_rx).await {
        Ok(Ok(Ok(()))) => Ok(OutboundTtsSession {
            worker,
            worker_cancel,
            forward_cancel: pipeline_cancel.child_token(),
        }),
        Ok(Ok(Err(msg))) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(WORKER_JOIN_TIMEOUT, worker).await;
            Err(msg)
        }
        Ok(Err(_)) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(WORKER_JOIN_TIMEOUT, worker).await;
            Err("Soniox TTS worker stopped unexpectedly".into())
        }
        Err(_) => {
            worker_cancel.cancel();
            let _ = tokio::time::timeout(WORKER_JOIN_TIMEOUT, worker).await;
            Err("Soniox TTS connect timeout (15s)".into())
        }
    }
}
