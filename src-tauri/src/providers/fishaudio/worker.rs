use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::audio::pcm_crossfade::{PcmChunkBoundary, PlaybackPcmChunk};
use crate::audio::try_send_pcm_bounded;
use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::client::IntoClientRequest,
    tungstenite::http::header::AUTHORIZATION,
    tungstenite::http::{HeaderName, HeaderValue},
    tungstenite::Message,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::config::live_ws_url;
use super::protocol::{
    is_non_retryable_fishaudio_error, model_header_value, pack_flush_message, pack_start_message,
    pack_stop_message, pack_text_message, parse_server_message, user_message_for_fishaudio_error,
    FishAudioInitSettings, ParsedServer,
};
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceTtsStatus;

#[derive(Debug, Clone)]
pub struct FishAudioWorkerConfig {
    pub api_key: String,
    pub init_settings: FishAudioInitSettings,
}

enum SessionEnd {
    Done,
    Reset,
}

pub fn spawn_fishaudio_tts_worker(
    config: FishAudioWorkerConfig,
    cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_fishaudio_tts_worker(
            config,
            cmd_rx,
            audio_tx,
            pcm_drops,
            status_tx,
            turn_latency,
            cancel,
        )
        .await
        {
            warn!("fishaudio tts worker exited: {e:#}");
        }
    })
}

async fn run_fishaudio_tts_worker(
    config: FishAudioWorkerConfig,
    mut cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> Result<()> {
    let first_audio_ms = Arc::new(AtomicU64::new(0));
    let mut attempts = 0u32;
    let mut session_index = 0u32;
    loop {
        if cancel.is_cancelled() {
            break;
        }
        let reconnected = session_index > 0;
        match run_session(
            &config,
            &mut cmd_rx,
            &audio_tx,
            &pcm_drops,
            &status_tx,
            &turn_latency,
            &first_audio_ms,
            &cancel,
            reconnected,
        )
        .await
        {
            Ok(SessionEnd::Done) => break,
            Ok(SessionEnd::Reset) => {
                attempts = 0;
                session_index += 1;
                continue;
            }
            Err(err) => {
                let raw = err.to_string();
                if cancel.is_cancelled() {
                    break;
                }
                if is_non_retryable_fishaudio_error(&raw) {
                    let message = user_message_for_fishaudio_error(&raw);
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                attempts += 1;
                if attempts >= 3 {
                    let message = user_message_for_fishaudio_error(&raw);
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                warn!("fishaudio session error (attempt {attempts}): {err:#}");
                let backoff = Duration::from_secs(1u64 << attempts.saturating_sub(1).min(2));
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }
            }
        }
    }
    info!("fishaudio tts worker shut down cleanly");
    Ok(())
}

async fn run_session(
    config: &FishAudioWorkerConfig,
    cmd_rx: &mut mpsc::Receiver<TtsTextCommand>,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
    cancel: &CancellationToken,
    reconnected: bool,
) -> Result<SessionEnd> {
    let url = live_ws_url();
    let mut request = url.into_client_request().context("fishaudio ws request")?;
    let auth = format!("Bearer {}", config.api_key.trim());
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth).context("fishaudio auth header")?,
    );
    request.headers_mut().insert(
        HeaderName::from_static("model"),
        HeaderValue::from_static(model_header_value(&config.init_settings.model_id)),
    );

    let (ws, _) = tokio::time::timeout(Duration::from_secs(15), connect_async(request))
        .await
        .map_err(|_| anyhow!("fishaudio connect timeout"))?
        .map_err(|e| anyhow!("fishaudio connect failed: {e}"))?;

    let (mut write, mut read) = ws.split();
    write
        .send(Message::Binary(pack_start_message(&config.init_settings)?))
        .await
        .context("fishaudio start send")?;
    crate::runtime::control_channel::try_send_control(
        status_tx,
        VoiceTtsStatus::Ready,
        "tts-status",
    );
    if reconnected {
        debug!("fishaudio tts websocket reconnected");
    } else {
        info!("fishaudio tts websocket ready");
    }

    let mut coalesce_buf: Vec<i16> = Vec::new();
    let flush_coalesce = |buf: &mut Vec<i16>,
                          boundary: PcmChunkBoundary,
                          audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
                          pcm_drops: &AtomicU64,
                          status_tx: &mpsc::Sender<VoiceTtsStatus>|
     -> Result<()> {
        if buf.is_empty() {
            if boundary == PcmChunkBoundary::SegmentEnd {
                let chunk = PlaybackPcmChunk {
                    samples: Vec::new(),
                    boundary: PcmChunkBoundary::SegmentEnd,
                };
                if !try_send_pcm_bounded(audio_tx, chunk, pcm_drops) && audio_tx.is_closed() {
                    warn!("fishaudio audio playback channel closed");
                    crate::runtime::control_channel::try_send_control(
                        status_tx,
                        VoiceTtsStatus::Degraded {
                            message: "Custom voice playback unavailable — Stop and Start to retry."
                                .into(),
                        },
                        "tts-status",
                    );
                    return Err(anyhow!("playback channel closed"));
                }
            }
            return Ok(());
        }
        let chunk = PlaybackPcmChunk {
            samples: std::mem::take(buf),
            boundary,
        };
        if !try_send_pcm_bounded(audio_tx, chunk, pcm_drops) && audio_tx.is_closed() {
            warn!("fishaudio audio playback channel closed");
            crate::runtime::control_channel::try_send_control(
                status_tx,
                VoiceTtsStatus::Degraded {
                    message: "Custom voice playback unavailable — Stop and Start to retry.".into(),
                },
                "tts-status",
            );
            return Err(anyhow!("playback channel closed"));
        }
        Ok(())
    };

    let mut deliver_pcm = |parsed: super::protocol::ParsedAudio, is_final: bool| -> Result<()> {
        let boundary = if is_final {
            PcmChunkBoundary::SegmentEnd
        } else {
            PcmChunkBoundary::Continuation
        };
        if !parsed.samples.is_empty() {
            debug!(samples = parsed.samples.len(), "fishaudio received audio");
            if first_audio_ms.load(Ordering::Relaxed) == 0 {
                let now = crate::audio::monotonic_ms();
                first_audio_ms.store(now, Ordering::Relaxed);
                turn_latency.record_first_audio();
            }
            coalesce_buf.extend(parsed.samples);
        }
        if is_final {
            return flush_coalesce(&mut coalesce_buf, boundary, audio_tx, pcm_drops, status_tx);
        }
        if coalesce_buf.len() >= crate::voice::config::EL_PCM_COALESCE_MIN_SAMPLES {
            return flush_coalesce(
                &mut coalesce_buf,
                PcmChunkBoundary::Continuation,
                audio_tx,
                pcm_drops,
                status_tx,
            );
        }
        Ok(())
    };

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = flush_coalesce(
                    &mut coalesce_buf,
                    PcmChunkBoundary::SegmentEnd,
                    audio_tx,
                    pcm_drops,
                    status_tx,
                );
                let _ = write.send(Message::Binary(pack_stop_message()?)).await;
                let _ = write.close().await;
                info!("fishaudio tts websocket closed on cancel");
                return Ok(SessionEnd::Done);
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(TtsTextCommand::AppendDelta {
                        text,
                        trigger_generation,
                    }) if !text.is_empty() => {
                        write
                            .send(Message::Binary(pack_text_message(&text)?))
                            .await
                            .context("fishaudio text send")?;
                        if trigger_generation {
                            write
                                .send(Message::Binary(pack_flush_message()?))
                                .await
                                .context("fishaudio trigger flush")?;
                        }
                    }
                    Some(TtsTextCommand::Flush) => {
                        write
                            .send(Message::Binary(pack_flush_message()?))
                            .await
                            .context("fishaudio flush send")?;
                    }
                    Some(TtsTextCommand::Reset) => {
                        let _ = flush_coalesce(
                            &mut coalesce_buf,
                            PcmChunkBoundary::SegmentEnd,
                            audio_tx,
                            pcm_drops,
                            status_tx,
                        );
                        debug!("fishaudio reset session");
                        let _ = write.send(Message::Binary(pack_stop_message()?)).await;
                        let _ = write.close().await;
                        return Ok(SessionEnd::Reset);
                    }
                    None => {
                        let _ = flush_coalesce(
                            &mut coalesce_buf,
                            PcmChunkBoundary::SegmentEnd,
                            audio_tx,
                            pcm_drops,
                            status_tx,
                        );
                        let _ = write.send(Message::Binary(pack_stop_message()?)).await;
                        return Ok(SessionEnd::Done);
                    }
                    _ => {}
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Binary(bytes))) => {
                        match parse_server_message(&bytes) {
                            Some(ParsedServer::Audio(parsed)) => {
                                if deliver_pcm(parsed, false).is_err() {
                                    return Ok(SessionEnd::Done);
                                }
                            }
                            Some(ParsedServer::Finish { .. }) => {
                                if deliver_pcm(
                                    super::protocol::ParsedAudio {
                                        samples: Vec::new(),
                                        is_final: true,
                                    },
                                    true,
                                )
                                .is_err()
                                {
                                    return Ok(SessionEnd::Done);
                                }
                            }
                            Some(ParsedServer::Error(err)) => {
                                warn!("fishaudio server error: {err}");
                                return Err(anyhow!("fishaudio server error: {err}"));
                            }
                            Some(ParsedServer::Ignored) | None => {}
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Some(ParsedServer::Error(err)) = parse_server_message(text.as_bytes())
                        {
                            return Err(anyhow!("fishaudio server error: {err}"));
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await.context("fishaudio pong")?;
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        return Err(anyhow!(
                            "fishaudio closed connection: {}",
                            frame.map(|f| f.reason.to_string()).unwrap_or_default()
                        ));
                    }
                    None => return Err(anyhow!("fishaudio websocket stream ended")),
                    Some(Err(e)) => return Err(anyhow!("fishaudio read error: {e}")),
                }
            }
        }
    }
}
