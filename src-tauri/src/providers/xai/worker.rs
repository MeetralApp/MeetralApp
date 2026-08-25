use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::audio::try_send_pcm_bounded;
use crate::audio::PlaybackPcmChunk;
use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest,
    tungstenite::http::header::AUTHORIZATION, tungstenite::http::HeaderValue, tungstenite::Message,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::config::live_ws_url;
use super::pack::{SpeakableUnit, UtteranceBuf};
use super::playback::PcmCoalesce;
use super::protocol::{
    is_non_retryable_xai_error, pack_text_clear, pack_text_delta, pack_text_done,
    parse_server_message, user_message_for_xai_error, ParsedServer, XaiInitSettings,
};
use super::schedule::SocketQueue;
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceTtsStatus;

type XaiWs =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

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
    let mut latency = settings.optimize_streaming_latency();
    let mut url = live_ws_url(
        &settings.language,
        settings.voice_id.trim(),
        settings.clamped_speed(),
        latency,
    );
    let ws = match connect_ws(&config.api_key, &url).await {
        Ok(ws) => ws,
        Err(err) if latency == 2 && looks_like_latency_reject(&err.to_string()) => {
            warn!("xai latency=2 rejected; falling back to 1");
            latency = 1;
            url = live_ws_url(
                &settings.language,
                settings.voice_id.trim(),
                settings.clamped_speed(),
                latency,
            );
            connect_ws(&config.api_key, &url).await?
        }
        Err(err) => return Err(err),
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

    let utterance = UtteranceBuf::new();
    let mut queue = SocketQueue::new();
    let mut pcm = PcmCoalesce::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = emit_chunks(
                    pcm.reset(),
                    audio_tx,
                    pcm_drops,
                    status_tx,
                    turn_latency,
                    first_audio_ms,
                );
                let _ = write.close().await;
                info!("xai tts websocket closed on cancel");
                return Ok(SessionEnd::Done);
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(TtsTextCommand::AppendDelta { text, .. }) if !text.is_empty() => {
                        start_next(
                            queue.push_units(utterance.push_delta(&text)),
                            &mut write,
                        )
                        .await?;
                    }
                    Some(TtsTextCommand::Flush) => {
                        start_next(
                            queue.push_units(utterance.flush()),
                            &mut write,
                        )
                        .await?;
                    }
                    Some(TtsTextCommand::Reset) => {
                        queue.begin_reset();
                        if emit_chunks(
                            pcm.reset(),
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
                        debug!("xai clear utterance (reset)");
                        write
                            .send(Message::Text(pack_text_clear()?))
                            .await
                            .context("xai ws send")?;
                    }
                    None => {
                        let _ = emit_chunks(
                            pcm.reset(),
                            audio_tx,
                            pcm_drops,
                            status_tx,
                            turn_latency,
                            first_audio_ms,
                        );
                        let _ = write.close().await;
                        return Ok(SessionEnd::Done);
                    }
                    _ => {}
                }
            }
            msg = read.next() => {
                match on_socket_message(
                    msg,
                    &mut queue,
                    &mut pcm,
                    &mut write,
                    audio_tx,
                    pcm_drops,
                    status_tx,
                    turn_latency,
                    first_audio_ms,
                )
                .await?
                {
                    ReadOutcome::Continue => {}
                    ReadOutcome::Done => return Ok(SessionEnd::Done),
                }
            }
        }
    }
}

enum ReadOutcome {
    Continue,
    Done,
}

async fn on_socket_message<W>(
    msg: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    queue: &mut SocketQueue,
    pcm: &mut PcmCoalesce,
    write: &mut W,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
) -> Result<ReadOutcome>
where
    W: SinkExt<Message> + Unpin,
    <W as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    match msg {
        Some(Ok(Message::Text(text))) => match parse_server_message(&text) {
            Some(ParsedServer::Audio(parsed)) => {
                if queue.is_generating()
                    && emit_chunks(
                        pcm.push(parsed.samples),
                        audio_tx,
                        pcm_drops,
                        status_tx,
                        turn_latency,
                        first_audio_ms,
                    )
                    .is_err()
                {
                    return Ok(ReadOutcome::Done);
                }
                Ok(ReadOutcome::Continue)
            }
            Some(ParsedServer::AudioDone) => {
                if queue.is_generating()
                    && emit_chunks(
                        pcm.finish_segment(),
                        audio_tx,
                        pcm_drops,
                        status_tx,
                        turn_latency,
                        first_audio_ms,
                    )
                    .is_err()
                {
                    return Ok(ReadOutcome::Done);
                }
                start_next(queue.on_audio_done(), write).await?;
                Ok(ReadOutcome::Continue)
            }
            Some(ParsedServer::AudioClear) => {
                debug!("xai audio.clear received");
                start_next(queue.on_audio_clear(), write).await?;
                Ok(ReadOutcome::Continue)
            }
            Some(ParsedServer::Error(err)) => {
                warn!("xai server error: {err}");
                Err(anyhow!("xai server error: {err}"))
            }
            Some(ParsedServer::Ignored) | None => Ok(ReadOutcome::Continue),
        },
        Some(Ok(Message::Binary(_))) => Ok(ReadOutcome::Continue),
        Some(Ok(Message::Ping(data))) => {
            write.send(Message::Pong(data)).await.context("xai pong")?;
            Ok(ReadOutcome::Continue)
        }
        Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => Ok(ReadOutcome::Continue),
        Some(Ok(Message::Close(frame))) => {
            let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
            Err(anyhow!("xai closed connection: {reason}"))
        }
        None => Err(anyhow!("xai websocket stream ended")),
        Some(Err(e)) => Err(anyhow!("xai read error: {e}")),
    }
}

async fn start_next<W>(unit: Option<SpeakableUnit>, write: &mut W) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    <W as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    let Some(unit) = unit else {
        return Ok(());
    };
    debug!(chars = unit.text.chars().count(), "xai start utterance");
    let delta = pack_text_delta(&unit.text)?;
    let done = pack_text_done()?;
    write
        .send(Message::Text(delta))
        .await
        .context("xai ws send")?;
    write
        .send(Message::Text(done))
        .await
        .context("xai ws send")?;
    Ok(())
}

fn emit_chunks(
    chunks: Vec<PlaybackPcmChunk>,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
) -> Result<()> {
    for chunk in chunks {
        if !chunk.samples.is_empty() && first_audio_ms.load(Ordering::Relaxed) == 0 {
            let now = crate::audio::monotonic_ms();
            first_audio_ms.store(now, Ordering::Relaxed);
            turn_latency.record_first_audio();
        }
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
    }
    Ok(())
}

async fn connect_ws(api_key: &str, url: &str) -> Result<XaiWs> {
    let mut request = url.into_client_request().context("xai ws request")?;
    let auth = format!("Bearer {}", api_key.trim());
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth).context("xai auth header")?,
    );
    let connect_result =
        tokio::time::timeout(Duration::from_secs(15), connect_async(request)).await;
    match connect_result {
        Ok(Ok((ws, _))) => Ok(ws),
        Ok(Err(e)) => Err(anyhow!("xai connect failed: {e}")),
        Err(_) => Err(anyhow!("xai connect timeout")),
    }
}

fn looks_like_latency_reject(msg: &str) -> bool {
    msg.contains("400") || msg.contains("optimize_streaming_latency")
}
