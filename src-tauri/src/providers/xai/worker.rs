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
    tungstenite::http::HeaderValue,
    tungstenite::Message,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::config::live_ws_url;
use super::protocol::{
    is_non_retryable_xai_error, pack_text_clear, pack_text_delta, pack_text_done,
    parse_server_message, user_message_for_xai_error, ParsedServer, XaiInitSettings, XaiTurnGate,
    XaiWireAction,
};
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceTtsStatus;

#[derive(Debug, Clone)]
pub struct XaiWorkerConfig {
    pub api_key: String,
    pub init_settings: XaiInitSettings,
}

enum SessionEnd {
    Done,
}

pub fn spawn_xai_tts_worker(
    config: XaiWorkerConfig,
    cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_xai_tts_worker(
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
            warn!("xai tts worker exited: {e:#}");
        }
    })
}

async fn run_xai_tts_worker(
    config: XaiWorkerConfig,
    mut cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> Result<()> {
    let first_audio_ms = Arc::new(AtomicU64::new(0));
    let mut attempts = 0u32;
    let mut reconnected = false;
    loop {
        if cancel.is_cancelled() {
            break;
        }
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
            Err(err) => {
                let raw = err.to_string();
                if cancel.is_cancelled() {
                    break;
                }
                if is_non_retryable_xai_error(&raw) {
                    let message = user_message_for_xai_error(&raw);
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                attempts += 1;
                if attempts >= 3 {
                    let message = user_message_for_xai_error(&raw);
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                warn!("xai session error (attempt {attempts}): {err:#}");
                reconnected = true;
                let backoff = Duration::from_secs(1u64 << attempts.saturating_sub(1).min(2));
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }
            }
        }
    }
    info!("xai tts worker shut down cleanly");
    Ok(())
}

async fn run_session(
    config: &XaiWorkerConfig,
    cmd_rx: &mut mpsc::Receiver<TtsTextCommand>,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
    cancel: &CancellationToken,
    reconnected: bool,
) -> Result<SessionEnd> {
    let settings = &config.init_settings;
    let mut url = live_ws_url(
        &settings.language,
        settings.voice_id.trim(),
        settings.clamped_speed(),
        settings.optimize_streaming_latency(),
    );
    let mut request = url
        .as_str()
        .into_client_request()
        .context("xai ws request")?;
    let auth = format!("Bearer {}", config.api_key.trim());
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth).context("xai auth header")?,
    );

    let connect_result = tokio::time::timeout(Duration::from_secs(15), connect_async(request)).await;
    let (ws, _) = match connect_result {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => {
            let msg = e.to_string();
            // Docs: latency=2 may 400 on some schema revisions — retry once with 1.
            if settings.optimize_streaming_latency() == 2
                && (msg.contains("400") || msg.contains("optimize_streaming_latency"))
            {
                warn!("xai latency=2 rejected; falling back to 1");
                url = live_ws_url(
                    &settings.language,
                    settings.voice_id.trim(),
                    settings.clamped_speed(),
                    1,
                );
                let mut request = url
                    .as_str()
                    .into_client_request()
                    .context("xai ws request retry")?;
                request.headers_mut().insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&auth).context("xai auth header")?,
                );
                tokio::time::timeout(Duration::from_secs(15), connect_async(request))
                    .await
                    .map_err(|_| anyhow!("xai connect timeout"))?
                    .map_err(|e| anyhow!("xai connect failed: {e}"))?
            } else {
                return Err(anyhow!("xai connect failed: {msg}"));
            }
        }
        Err(_) => return Err(anyhow!("xai connect timeout")),
    };

    let (mut write, mut read) = ws.split();
    crate::runtime::control_channel::try_send_control(
        status_tx,
        VoiceTtsStatus::Ready,
        "tts-status",
    );
    if reconnected {
        debug!("xai tts websocket reconnected");
    } else {
        info!("xai tts websocket ready");
    }

    let mut coalesce_buf: Vec<i16> = Vec::new();
    let mut turn_gate = XaiTurnGate::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = flush_coalesce_buf(
                    &mut coalesce_buf,
                    PcmChunkBoundary::SegmentEnd,
                    audio_tx,
                    pcm_drops,
                    status_tx,
                );
                let _ = write.close().await;
                info!("xai tts websocket closed on cancel");
                return Ok(SessionEnd::Done);
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(TtsTextCommand::AppendDelta { text, .. }) if !text.is_empty() => {
                        send_wire_actions(&mut write, turn_gate.on_delta(text)).await?;
                    }
                    Some(TtsTextCommand::Flush) => {
                        send_wire_actions(&mut write, turn_gate.on_flush()).await?;
                    }
                    Some(TtsTextCommand::Reset) => {
                        let _ = flush_coalesce_buf(
                            &mut coalesce_buf,
                            PcmChunkBoundary::SegmentEnd,
                            audio_tx,
                            pcm_drops,
                            status_tx,
                        );
                        debug!("xai clear utterance (reset)");
                        send_wire_actions(&mut write, turn_gate.on_reset()).await?;
                        // Stay on the same socket; wait for audio.clear below.
                    }
                    None => {
                        let _ = flush_coalesce_buf(
                            &mut coalesce_buf,
                            PcmChunkBoundary::SegmentEnd,
                            audio_tx,
                            pcm_drops,
                            status_tx,
                        );
                        let _ = write.close().await;
                        return Ok(SessionEnd::Done);
                    }
                    _ => {}
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match parse_server_message(&text) {
                            Some(ParsedServer::Audio(parsed)) => {
                                if deliver_pcm_samples(
                                    &mut coalesce_buf,
                                    parsed.samples,
                                    false,
                                    audio_tx,
                                    pcm_drops,
                                    status_tx,
                                    turn_latency,
                                    first_audio_ms,
                                )
                                .is_err()
                                {
                                    return Ok(SessionEnd::Done);
                                }
                            }
                            Some(ParsedServer::AudioDone) => {
                                if deliver_pcm_samples(
                                    &mut coalesce_buf,
                                    Vec::new(),
                                    true,
                                    audio_tx,
                                    pcm_drops,
                                    status_tx,
                                    turn_latency,
                                    first_audio_ms,
                                )
                                .is_err()
                                {
                                    return Ok(SessionEnd::Done);
                                }
                                send_wire_actions(&mut write, turn_gate.on_audio_done()).await?;
                            }
                            Some(ParsedServer::AudioClear) => {
                                coalesce_buf.clear();
                                turn_gate.on_audio_clear();
                                debug!("xai audio.clear received");
                            }
                            Some(ParsedServer::Error(err)) => {
                                warn!("xai server error: {err}");
                                return Err(anyhow!("xai server error: {err}"));
                            }
                            Some(ParsedServer::Ignored) | None => {}
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await.context("xai pong")?;
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        return Err(anyhow!(
                            "xai closed connection: {}",
                            frame.map(|f| f.reason.to_string()).unwrap_or_default()
                        ));
                    }
                    None => return Err(anyhow!("xai websocket stream ended")),
                    Some(Err(e)) => return Err(anyhow!("xai read error: {e}")),
                }
            }
        }
    }
}

async fn send_wire_actions<S>(write: &mut S, actions: Vec<XaiWireAction>) -> Result<()>
where
    S: SinkExt<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    for action in actions {
        let payload = match action {
            XaiWireAction::TextDelta(text) => pack_text_delta(&text)?,
            XaiWireAction::TextDone => pack_text_done()?,
            XaiWireAction::TextClear => pack_text_clear()?,
        };
        write
            .send(Message::Text(payload.into()))
            .await
            .context("xai ws send")?;
    }
    Ok(())
}

fn flush_coalesce_buf(
    buf: &mut Vec<i16>,
    boundary: PcmChunkBoundary,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
) -> Result<()> {
    if buf.is_empty() {
        if boundary == PcmChunkBoundary::SegmentEnd {
            let chunk = PlaybackPcmChunk {
                samples: Vec::new(),
                boundary: PcmChunkBoundary::SegmentEnd,
            };
            if !try_send_pcm_bounded(audio_tx, chunk, pcm_drops) && audio_tx.is_closed() {
                warn!("xai audio playback channel closed");
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
        warn!("xai audio playback channel closed");
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
}

fn deliver_pcm_samples(
    coalesce_buf: &mut Vec<i16>,
    samples: Vec<i16>,
    is_final: bool,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
) -> Result<()> {
    let boundary = if is_final {
        PcmChunkBoundary::SegmentEnd
    } else {
        PcmChunkBoundary::Continuation
    };
    if !samples.is_empty() {
        debug!(samples = samples.len(), "xai received audio");
        if first_audio_ms.load(Ordering::Relaxed) == 0 {
            let now = crate::audio::monotonic_ms();
            first_audio_ms.store(now, Ordering::Relaxed);
            turn_latency.record_first_audio();
        }
        coalesce_buf.extend(samples);
    }
    if is_final {
        return flush_coalesce_buf(coalesce_buf, boundary, audio_tx, pcm_drops, status_tx);
    }
    if coalesce_buf.len() >= crate::voice::config::EL_PCM_COALESCE_MIN_SAMPLES {
        return flush_coalesce_buf(
            coalesce_buf,
            PcmChunkBoundary::Continuation,
            audio_tx,
            pcm_drops,
            status_tx,
        );
    }
    Ok(())
}
