use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::audio::pcm_crossfade::PlaybackPcmChunk;
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

use super::config::{live_ws_url, SOCKET_COUNT};
use super::pack::{SpeakableUnit, UtteranceBuf};
use super::playback::OrderedPlayback;
use super::protocol::{
    is_non_retryable_xai_error, pack_text_clear, pack_text_delta, pack_text_done,
    parse_server_message, user_message_for_xai_error, ParsedServer, XaiInitSettings,
};
use super::schedule::{SlotAssigner, SlotFinish};
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceTtsStatus;

type XaiWs = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

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
    let (ws0, ws1) = match connect_sockets(&config.api_key, &url, SOCKET_COUNT >= 2).await {
        Ok(pair) => pair,
        Err(err) if latency == 2 && looks_like_latency_reject(&err.to_string()) => {
            warn!("xai latency=2 rejected; falling back to 1");
            latency = 1;
            url = live_ws_url(
                &settings.language,
                settings.voice_id.trim(),
                settings.clamped_speed(),
                latency,
            );
            connect_sockets(&config.api_key, &url, SOCKET_COUNT >= 2).await?
        }
        Err(err) => return Err(err),
    };

    let slot_count = if ws1.is_some() { 2 } else { 1 };
    let (mut w0, mut r0) = ws0.split();
    let (mut w1, mut r1) = match ws1 {
        Some(ws) => {
            let (w, r) = ws.split();
            (Some(w), Some(r))
        }
        None => (None, None),
    };
    let mut r0_live = true;
    let mut r1_live = r1.is_some();

    crate::runtime::control_channel::try_send_control(
        status_tx,
        VoiceTtsStatus::Ready,
        "tts-status",
    );
    if reconnected {
        debug!(sockets = slot_count, "xai tts websocket reconnected");
    } else {
        info!(sockets = slot_count, "xai tts websocket ready");
    }

    let mut utterance = UtteranceBuf::new();
    let mut assigner = SlotAssigner::new(slot_count);
    let mut playback = OrderedPlayback::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = emit_chunks(
                    playback.reset(),
                    audio_tx,
                    pcm_drops,
                    status_tx,
                    turn_latency,
                    first_audio_ms,
                );
                let _ = w0.close().await;
                if let Some(w) = w1.as_mut() {
                    let _ = w.close().await;
                }
                info!("xai tts websocket closed on cancel");
                return Ok(SessionEnd::Done);
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(TtsTextCommand::AppendDelta { text, .. }) if !text.is_empty() => {
                        dispatch_new(
                            utterance.push_delta(&text),
                            &mut assigner,
                            &mut w0,
                            &mut w1,
                            &mut r0_live,
                            &mut r1_live,
                        )
                        .await?;
                    }
                    Some(TtsTextCommand::Flush) => {
                        dispatch_new(
                            utterance.flush(),
                            &mut assigner,
                            &mut w0,
                            &mut w1,
                            &mut r0_live,
                            &mut r1_live,
                        )
                        .await?;
                    }
                    Some(TtsTextCommand::Reset) => {
                        utterance.reset();
                        assigner.begin_reset();
                        if emit_chunks(
                            playback.reset(),
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
                        send_clear_all(&mut w0, &mut w1, r0_live, r1_live).await?;
                    }
                    None => {
                        let _ = emit_chunks(
                            playback.reset(),
                            audio_tx,
                            pcm_drops,
                            status_tx,
                            turn_latency,
                            first_audio_ms,
                        );
                        let _ = w0.close().await;
                        if let Some(w) = w1.as_mut() {
                            let _ = w.close().await;
                        }
                        return Ok(SessionEnd::Done);
                    }
                    _ => {}
                }
            }
            msg = r0.next(), if r0_live => {
                match on_socket_message(
                    0,
                    msg,
                    &mut assigner,
                    &mut playback,
                    &mut w0,
                    &mut w1,
                    &mut r0_live,
                    &mut r1_live,
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
            msg = r1.as_mut().unwrap().next(), if r1_live => {
                match on_socket_message(
                    1,
                    msg,
                    &mut assigner,
                    &mut playback,
                    &mut w0,
                    &mut w1,
                    &mut r0_live,
                    &mut r1_live,
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

async fn on_socket_message<W0, W1>(
    slot: usize,
    msg: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    assigner: &mut SlotAssigner,
    playback: &mut OrderedPlayback,
    w0: &mut W0,
    w1: &mut Option<W1>,
    r0_live: &mut bool,
    r1_live: &mut bool,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
) -> Result<ReadOutcome>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    match msg {
        Some(Ok(Message::Text(text))) => match parse_server_message(&text) {
            Some(ParsedServer::Audio(parsed)) => {
                if let Some(seq) = assigner.slot_seq(slot) {
                    if emit_chunks(
                        playback.push_samples(seq, parsed.samples),
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
                }
                Ok(ReadOutcome::Continue)
            }
            Some(ParsedServer::AudioDone) => {
                let finish = assigner.on_audio_done(slot);
                apply_slot_finish(
                    finish,
                    assigner,
                    playback,
                    w0,
                    w1,
                    r0_live,
                    r1_live,
                    audio_tx,
                    pcm_drops,
                    status_tx,
                    turn_latency,
                    first_audio_ms,
                )
                .await
            }
            Some(ParsedServer::AudioClear) => {
                debug!(slot, "xai audio.clear received");
                let finish = assigner.on_audio_clear(slot);
                apply_slot_finish(
                    finish,
                    assigner,
                    playback,
                    w0,
                    w1,
                    r0_live,
                    r1_live,
                    audio_tx,
                    pcm_drops,
                    status_tx,
                    turn_latency,
                    first_audio_ms,
                )
                .await
            }
            Some(ParsedServer::Error(err)) => {
                warn!("xai server error: {err}");
                Err(anyhow!("xai server error: {err}"))
            }
            Some(ParsedServer::Ignored) | None => Ok(ReadOutcome::Continue),
        },
        Some(Ok(Message::Binary(_))) => Ok(ReadOutcome::Continue),
        Some(Ok(Message::Ping(data))) => {
            match slot {
                0 => w0.send(Message::Pong(data)).await.context("xai pong")?,
                1 => {
                    w1.as_mut()
                        .ok_or_else(|| anyhow!("xai slot 1 missing"))?
                        .send(Message::Pong(data))
                        .await
                        .context("xai pong")?;
                }
                _ => {}
            }
            Ok(ReadOutcome::Continue)
        }
        Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => Ok(ReadOutcome::Continue),
        Some(Ok(Message::Close(frame))) => {
            let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
            drop_and_failover(
                slot,
                anyhow!("xai closed connection: {reason}"),
                assigner,
                w0,
                w1,
                r0_live,
                r1_live,
            )
            .await
        }
        None => drop_and_failover(
            slot,
            anyhow!("xai websocket stream ended"),
            assigner,
            w0,
            w1,
            r0_live,
            r1_live,
        )
        .await,
        Some(Err(e)) => {
            drop_and_failover(
                slot,
                anyhow!("xai read error: {e}"),
                assigner,
                w0,
                w1,
                r0_live,
                r1_live,
            )
            .await
        }
    }
}

async fn apply_slot_finish<W0, W1>(
    finish: SlotFinish,
    assigner: &mut SlotAssigner,
    playback: &mut OrderedPlayback,
    w0: &mut W0,
    w1: &mut Option<W1>,
    r0_live: &mut bool,
    r1_live: &mut bool,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
) -> Result<ReadOutcome>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    let next = match finish {
        SlotFinish::Completed { seq, next } => {
            if emit_chunks(
                playback.mark_done(seq),
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
            next
        }
        SlotFinish::Cancelled { next } => next,
        SlotFinish::Ignored => return Ok(ReadOutcome::Continue),
    };
    if let Some(pair) = next {
        send_assigned(vec![pair], assigner, w0, w1, r0_live, r1_live).await?;
    }
    Ok(ReadOutcome::Continue)
}

async fn drop_and_failover<W0, W1>(
    slot: usize,
    err: anyhow::Error,
    assigner: &mut SlotAssigner,
    w0: &mut W0,
    w1: &mut Option<W1>,
    r0_live: &mut bool,
    r1_live: &mut bool,
) -> Result<ReadOutcome>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    warn!(slot, "{err:#}");
    drop_slot(slot, w1, r0_live, r1_live);
    let assigned = assigner.on_socket_dead(slot);
    if assigner.all_dead() {
        return Err(err);
    }
    send_assigned(assigned, assigner, w0, w1, r0_live, r1_live).await?;
    Ok(ReadOutcome::Continue)
}

async fn dispatch_new<W0, W1>(
    units: Vec<SpeakableUnit>,
    assigner: &mut SlotAssigner,
    w0: &mut W0,
    w1: &mut Option<W1>,
    r0_live: &mut bool,
    r1_live: &mut bool,
) -> Result<()>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    let assigned = assigner.push_units(units);
    send_assigned(assigned, assigner, w0, w1, r0_live, r1_live).await
}

async fn send_assigned<W0, W1>(
    mut assigned: Vec<(usize, SpeakableUnit)>,
    assigner: &mut SlotAssigner,
    w0: &mut W0,
    w1: &mut Option<W1>,
    r0_live: &mut bool,
    r1_live: &mut bool,
) -> Result<()>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    while !assigned.is_empty() {
        let mut retry = Vec::new();
        for (slot, unit) in assigned {
            debug!(
                slot,
                seq = unit.seq,
                chars = unit.text.chars().count(),
                "xai start utterance"
            );
            if let Err(err) = send_utterance(w0, w1, slot, &unit.text).await {
                warn!(slot, "xai utterance send failed: {err:#}");
                drop_slot(slot, w1, r0_live, r1_live);
                retry.extend(assigner.on_socket_dead(slot));
                if assigner.all_dead() {
                    return Err(err);
                }
            }
        }
        assigned = retry;
    }
    Ok(())
}

async fn send_utterance<W0, W1>(
    w0: &mut W0,
    w1: &mut Option<W1>,
    slot: usize,
    text: &str,
) -> Result<()>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    let delta = pack_text_delta(text)?;
    let done = pack_text_done()?;
    send_text(w0, w1, slot, delta).await?;
    send_text(w0, w1, slot, done).await
}

async fn send_clear_all<W0, W1>(
    w0: &mut W0,
    w1: &mut Option<W1>,
    r0_live: bool,
    r1_live: bool,
) -> Result<()>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    let payload = pack_text_clear()?;
    if r0_live {
        send_text(w0, w1, 0, payload.clone()).await?;
    }
    if r1_live {
        send_text(w0, w1, 1, payload).await?;
    }
    Ok(())
}

async fn send_text<W0, W1>(
    w0: &mut W0,
    w1: &mut Option<W1>,
    slot: usize,
    payload: String,
) -> Result<()>
where
    W0: SinkExt<Message> + Unpin,
    W1: SinkExt<Message> + Unpin,
    <W0 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
    <W1 as futures_util::Sink<Message>>::Error: std::error::Error + Send + Sync + 'static,
{
    match slot {
        0 => {
            w0.send(Message::Text(payload.into()))
                .await
                .context("xai ws send")?;
        }
        1 => {
            w1.as_mut()
                .ok_or_else(|| anyhow!("xai slot 1 missing"))?
                .send(Message::Text(payload.into()))
                .await
                .context("xai ws send")?;
        }
        _ => return Err(anyhow!("xai slot {slot} out of range")),
    }
    Ok(())
}

fn drop_slot<W1>(slot: usize, w1: &mut Option<W1>, r0_live: &mut bool, r1_live: &mut bool) {
    match slot {
        0 => *r0_live = false,
        1 => {
            *r1_live = false;
            *w1 = None;
        }
        _ => {}
    }
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
                    message: "Custom voice playback unavailable — Stop and Start to retry."
                        .into(),
                },
                "tts-status",
            );
            return Err(anyhow!("playback channel closed"));
        }
    }
    Ok(())
}

async fn connect_sockets(api_key: &str, url: &str, want_two: bool) -> Result<(XaiWs, Option<XaiWs>)> {
    if !want_two {
        return Ok((connect_one(api_key, url).await?, None));
    }
    let (a, b) = tokio::join!(connect_one(api_key, url), connect_one(api_key, url));
    match (a, b) {
        (Ok(first), Ok(second)) => Ok((first, Some(second))),
        (Ok(first), Err(err)) => {
            warn!("xai second socket failed, staying single-socket: {err:#}");
            Ok((first, None))
        }
        (Err(err), Ok(second)) => {
            warn!("xai first socket failed, using second: {err:#}");
            Ok((second, None))
        }
        (Err(first), Err(second)) => Err(first.context(format!("both xAI sockets failed ({second:#})"))),
    }
}

async fn connect_one(api_key: &str, url: &str) -> Result<XaiWs> {
    let mut request = url
        .into_client_request()
        .context("xai ws request")?;
    let auth = format!("Bearer {}", api_key.trim());
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth).context("xai auth header")?,
    );
    let connect_result = tokio::time::timeout(Duration::from_secs(15), connect_async(request)).await;
    match connect_result {
        Ok(Ok((ws, _))) => Ok(ws),
        Ok(Err(e)) => Err(anyhow!("xai connect failed: {e}")),
        Err(_) => Err(anyhow!("xai connect timeout")),
    }
}

fn looks_like_latency_reject(msg: &str) -> bool {
    msg.contains("400") || msg.contains("optimize_streaming_latency")
}
