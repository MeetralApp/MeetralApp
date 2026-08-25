use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::audio::try_send_pcm_bounded;
use crate::audio::PlaybackPcmChunk;
use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use uuid::Uuid;

use super::config::{tts_ws_url, SONIOX_PCM_COALESCE_MIN_SAMPLES, STREAM_IDLE_TEXT_END};
use super::protocol::{
    build_cancel_message, build_keepalive_message, build_text_message, build_tts_config_message,
    parse_tts_message,
};
use crate::voice::elevenlabs::latency::TurnLatencySlot;
use crate::voice::shared::debug;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceTtsStatus;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = futures_util::stream::SplitSink<WsStream, Message>;
type WsRead = futures_util::stream::SplitStream<WsStream>;

#[derive(Debug, Clone)]
pub struct SonioxTtsWorkerConfig {
    pub api_key: String,
    pub voice: String,
    pub model: String,
    pub language: String,
    pub speed: f32,
}

enum SessionEnd {
    Done,
    /// Drop WS and reconnect lazily on next work.
    Idle,
    /// Reconnect immediately with queued text.
    Reconnect(PendingWork),
}

struct PendingStream {
    text: String,
    flush: bool,
}

struct PendingWork {
    queue: VecDeque<PendingStream>,
}

fn queue_append_delta(queue: &mut VecDeque<PendingStream>, text: String) {
    if let Some(back) = queue.back_mut() {
        if !back.flush {
            back.text.push_str(&text);
            return;
        }
    }
    queue.push_back(PendingStream { text, flush: false });
}

fn queue_mark_flush(queue: &mut VecDeque<PendingStream>) {
    if let Some(back) = queue.back_mut() {
        back.flush = true;
    }
}

fn pending_work_from_queue(queue: VecDeque<PendingStream>) -> Option<PendingWork> {
    if queue.is_empty() {
        None
    } else {
        Some(PendingWork { queue })
    }
}

pub fn spawn_soniox_tts_worker(
    config: SonioxTtsWorkerConfig,
    cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_soniox_tts_worker(
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
            warn!("soniox tts worker exited: {e:#}");
        }
    })
}

async fn run_soniox_tts_worker(
    config: SonioxTtsWorkerConfig,
    mut cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> Result<()> {
    let first_audio_ms = Arc::new(AtomicU64::new(0));
    let mut attempts = 0u32;
    let mut seed_override: Option<PendingWork> = None;
    let mut announced_ready = false;

    loop {
        if cancel.is_cancelled() {
            break;
        }

        // Eager connect + auth (within ~10s) so first AppendDelta only sends text.
        let (ws, _) = tokio::time::timeout(Duration::from_secs(15), connect_async(tts_ws_url()))
            .await
            .map_err(|_| anyhow!("soniox tts connect timeout"))?
            .map_err(|e| anyhow!("soniox tts connect failed: {e}"))?;
        let (mut write, mut read) = ws.split();
        debug::log_soniox_tts("websocket_connected");

        if let Err(err) = warmup_authenticate(&mut write, &mut read, &config, &cancel).await {
            warn!("soniox tts warmup auth failed: {err:#}");
            attempts += 1;
            if attempts >= 3 {
                let message = format!("Soniox TTS unavailable: {err}");
                crate::runtime::control_channel::try_send_control(
                    &status_tx,
                    VoiceTtsStatus::Degraded { message },
                    "tts-status",
                );
                return Err(err);
            }
            let backoff = Duration::from_secs(1u64 << attempts.saturating_sub(1).min(2));
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(backoff) => {}
            }
            continue;
        }

        if !announced_ready {
            crate::runtime::control_channel::try_send_control(
                &status_tx,
                VoiceTtsStatus::Ready,
                "tts-status",
            );
            announced_ready = true;
            debug::log_soniox_tts("worker_ready_eager");
        }

        let seed = if let Some(seed) = seed_override.take() {
            seed
        } else {
            match wait_for_work_keepalive(&mut cmd_rx, &mut write, &cancel).await {
                Some(seed) => seed,
                None => break,
            }
        };

        match run_connected_session(
            &config,
            &mut cmd_rx,
            &audio_tx,
            &pcm_drops,
            &status_tx,
            &turn_latency,
            &first_audio_ms,
            &cancel,
            seed,
            write,
            read,
        )
        .await
        {
            Ok(SessionEnd::Done) => break,
            Ok(SessionEnd::Idle) => {
                attempts = 0;
            }
            Ok(SessionEnd::Reconnect(pending)) => {
                attempts = 0;
                seed_override = Some(pending);
            }
            Err(err) => {
                if cancel.is_cancelled() {
                    break;
                }
                attempts += 1;
                if attempts >= 3 {
                    let message = format!("Soniox TTS unavailable: {err}");
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                warn!("soniox tts session error (attempt {attempts}): {err:#}");
                let backoff = Duration::from_secs(1u64 << attempts.saturating_sub(1).min(2));
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }
            }
        }
    }
    debug::log_soniox_tts("worker_shutdown");
    Ok(())
}

/// Config + cancel a disposable stream so keepalive is valid before first speech.
async fn warmup_authenticate(
    write: &mut WsSink,
    read: &mut WsRead,
    config: &SonioxTtsWorkerConfig,
    cancel: &CancellationToken,
) -> Result<()> {
    let id = format!("tts-warmup-{}", Uuid::new_v4());
    write
        .send(Message::Text(build_tts_config_message(
            &config.api_key,
            &config.language,
            &config.voice,
            &id,
            &config.model,
            config.speed,
        )))
        .await
        .context("soniox tts warmup config")?;
    write
        .send(Message::Text(build_cancel_message(&id)))
        .await
        .context("soniox tts warmup cancel")?;
    debug::log_soniox_tts("warmup_auth_sent");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        tokio::select! {
                   biased;
                   _ = cancel.cancelled() => return Err(anyhow!("cancelled during warmup")),
                   _ = tokio::time::sleep_until(deadline) => {
        // Auth message was sent; proceed even if terminated is slow.
                       debug::log_soniox_tts("warmup_auth_timeout_continue");
                       return Ok(());
                   }
                   maybe_msg = read.next() => {
                       match maybe_msg {
                           Some(Ok(Message::Text(text))) => {
                               if let Some(parsed) = parse_tts_message(&text) {
                                   if parsed.terminated {
                                       debug::log_soniox_tts("warmup_auth_terminated");
                                       return Ok(());
                                   }
                                   if let Some(err) = parsed.error {
                                       if !is_recoverable_tts_error(&err) {
                                           return Err(anyhow!("soniox tts warmup error: {err}"));
                                       }
                                   }
                               }
                           }
                           Some(Ok(Message::Close(_))) | None => {
                               return Err(anyhow!("soniox tts warmup connection closed"));
                           }
                           Some(Ok(_)) => {}
                           Some(Err(e)) => return Err(e.into()),
                       }
                   }
               }
    }
}

/// Wait for first AppendDelta on an already-authenticated WebSocket.
async fn wait_for_work_keepalive(
    cmd_rx: &mut mpsc::Receiver<TtsTextCommand>,
    write: &mut WsSink,
    cancel: &CancellationToken,
) -> Option<PendingWork> {
    let keepalive_interval = Duration::from_secs(20);
    let mut keepalive_at = tokio::time::Instant::now() + keepalive_interval;
    let mut queue = VecDeque::new();
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return None,
            _ = tokio::time::sleep_until(keepalive_at) => {
                keepalive_at = tokio::time::Instant::now() + keepalive_interval;
                let _ = write.send(Message::Text(build_keepalive_message())).await;
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(TtsTextCommand::AppendDelta { text, .. }) if !text.is_empty() => {
                        queue_append_delta(&mut queue, text);
                        return Some(PendingWork { queue });
                    }
                    Some(TtsTextCommand::Flush) => {}
                    Some(TtsTextCommand::Reset) => queue.clear(),
                    Some(_) => {}
                    None => return None,
                }
            }
        }
    }
}

async fn run_connected_session(
    config: &SonioxTtsWorkerConfig,
    cmd_rx: &mut mpsc::Receiver<TtsTextCommand>,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
    cancel: &CancellationToken,
    seed: PendingWork,
    mut write: WsSink,
    mut read: WsRead,
) -> Result<SessionEnd> {
    let keepalive_interval = Duration::from_secs(20);
    let mut keepalive_at = tokio::time::Instant::now() + keepalive_interval;
    let mut stream_id: Option<String> = None;
    let mut stream_open = false;
    let mut awaiting_terminated = false;
    let mut terminate_deadline: Option<tokio::time::Instant> = None;
    let mut pending_queue: VecDeque<PendingStream> = seed.queue;
    let mut coalesce_buf: Vec<i16> = Vec::new();
    // Warmup already authenticated this connection.
    let mut authenticated = true;
    const IDLE_DISCONNECT: Duration = Duration::from_secs(90);
    let mut idle_deadline: Option<tokio::time::Instant> = None;
    let mut stream_idle_deadline: Option<tokio::time::Instant> = None;
    const TERMINATE_TIMEOUT: Duration = Duration::from_secs(15);

    let flush_coalesce = |buf: &mut Vec<i16>,
                          audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
                          pcm_drops: &AtomicU64|
     -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let chunk = PlaybackPcmChunk::new(std::mem::take(buf));
        if !try_send_pcm_bounded(audio_tx, chunk, pcm_drops) && audio_tx.is_closed() {
            return Err(anyhow!("playback channel closed"));
        }
        Ok(())
    };

    async fn open_and_send_text(
        write: &mut WsSink,
        config: &SonioxTtsWorkerConfig,
        stream_id: &mut Option<String>,
        stream_open: &mut bool,
        authenticated: &mut bool,
        first_audio_ms: &AtomicU64,
        text: &str,
    ) -> Result<()> {
        if !*stream_open {
            let id = format!("tts-{}", Uuid::new_v4());
            write
                .send(Message::Text(build_tts_config_message(
                    &config.api_key,
                    &config.language,
                    &config.voice,
                    &id,
                    &config.model,
                    config.speed,
                )))
                .await
                .context("soniox tts config send")?;
            *stream_id = Some(id.clone());
            *stream_open = true;
            *authenticated = true;
            // Re-arm per-stream immediate flush for the next first PCM chunk.
            first_audio_ms.store(0, Ordering::Relaxed);
            debug!(stream_id = %id, "soniox tts stream started");
        }
        if text.is_empty() {
            return Ok(());
        }
        let Some(id) = stream_id.as_ref() else {
            anyhow::bail!("soniox tts: stream marked open without stream id");
        };
        write
            .send(Message::Text(build_text_message(id, text, false)))
            .await
            .context("soniox tts text send")?;
        debug::log_soniox_tts_ws_text(text.chars().count());
        Ok(())
    }

    async fn send_text_end_and_await(
        write: &mut WsSink,
        stream_id: &Option<String>,
        stream_open: &mut bool,
        awaiting_terminated: &mut bool,
        terminate_deadline: &mut Option<tokio::time::Instant>,
    ) -> Result<()> {
        if !*stream_open {
            return Ok(());
        }
        let Some(id) = stream_id.as_ref() else {
            anyhow::bail!("soniox tts: stream marked open without stream id");
        };
        write
            .send(Message::Text(build_text_message(id, "", true)))
            .await
            .context("soniox tts flush send")?;
        *stream_open = false;
        *awaiting_terminated = true;
        *terminate_deadline = Some(tokio::time::Instant::now() + TERMINATE_TIMEOUT);
        debug::log_soniox_tts_ws_text_end();
        debug::log_soniox_tts("text_end_awaiting_terminated");
        debug!(stream_id = %id, "soniox tts text_end — awaiting terminated");
        Ok(())
    }

    // First utterance on this warm socket — open stream + send seed text.
    let seed_unit = pending_queue.pop_front().unwrap_or(PendingStream {
        text: String::new(),
        flush: false,
    });
    open_and_send_text(
        &mut write,
        config,
        &mut stream_id,
        &mut stream_open,
        &mut authenticated,
        first_audio_ms,
        &seed_unit.text,
    )
    .await?;
    if seed_unit.flush {
        send_text_end_and_await(
            &mut write,
            &stream_id,
            &mut stream_open,
            &mut awaiting_terminated,
            &mut terminate_deadline,
        )
        .await?;
        stream_idle_deadline = None;
    } else if stream_open {
        stream_idle_deadline = Some(tokio::time::Instant::now() + STREAM_IDLE_TEXT_END);
    }

    loop {
        tokio::select! {
                   biased;
                   _ = cancel.cancelled() => {
                       if let Some(ref id) = stream_id {
                           if stream_open || awaiting_terminated {
                               let _ = write.send(Message::Text(build_cancel_message(id))).await;
                           }
                       }
                       let _ = flush_coalesce(&mut coalesce_buf, audio_tx, pcm_drops);
                       let _ = write.close().await;
                       return Ok(SessionEnd::Done);
                   }
                   _ = async {
                       match terminate_deadline {
                           Some(deadline) => tokio::time::sleep_until(deadline).await,
                           None => std::future::pending::<()>().await,
                       }
                   }, if awaiting_terminated && terminate_deadline.is_some() => {
                       warn!("soniox tts terminated timeout — dropping websocket");
                       debug::log_soniox_tts("terminated_timeout_drop_ws");
                       let _ = flush_coalesce(&mut coalesce_buf, audio_tx, pcm_drops);
                       let _ = write.close().await;
                       if let Some(pending) = pending_work_from_queue(pending_queue) {
                           return Ok(SessionEnd::Reconnect(pending));
                       }
                       return Ok(SessionEnd::Idle);
                   }
                   _ = async {
                       match stream_idle_deadline {
                           Some(deadline) => tokio::time::sleep_until(deadline).await,
                           None => std::future::pending::<()>().await,
                       }
                   }, if stream_open && stream_idle_deadline.is_some() => {
                       debug::log_soniox_tts("stream_idle_text_end");
                       debug!("soniox tts stream idle — sending text_end (keep ws)");
                       stream_idle_deadline = None;
                       send_text_end_and_await(
                           &mut write,
                           &stream_id,
                           &mut stream_open,
                           &mut awaiting_terminated,
                           &mut terminate_deadline,
                       )
                       .await?;
                   }
                   _ = async {
                       match idle_deadline {
                           Some(deadline) => tokio::time::sleep_until(deadline).await,
                           None => std::future::pending::<()>().await,
                       }
                   }, if idle_deadline.is_some()
                       && !stream_open
                       && !awaiting_terminated
                       && pending_queue.is_empty() =>
                   {
                       debug::log_soniox_tts("idle_disconnect");
                       let _ = write.close().await;
                       return Ok(SessionEnd::Idle);
                   }
                   _ = tokio::time::sleep_until(keepalive_at), if authenticated => {
                       keepalive_at = tokio::time::Instant::now() + keepalive_interval;
                       let _ = write.send(Message::Text(build_keepalive_message())).await;
                   }
                   cmd = cmd_rx.recv() => {
                       keepalive_at = tokio::time::Instant::now() + keepalive_interval;
                       idle_deadline = None;
                       match cmd {
                           Some(TtsTextCommand::AppendDelta { text, .. }) if !text.is_empty() => {
                               if awaiting_terminated {
                                   queue_append_delta(&mut pending_queue, text);
                                   continue;
                               }
                               open_and_send_text(
                                   &mut write,
                                   config,
                                   &mut stream_id,
                                   &mut stream_open,
                                   &mut authenticated,
                                   first_audio_ms,
                                   &text,
                               )
                               .await?;
                               if stream_open {
                                   stream_idle_deadline =
                                       Some(tokio::time::Instant::now() + STREAM_IDLE_TEXT_END);
                               }
                           }
                           Some(TtsTextCommand::Flush) => {
                               if awaiting_terminated {
                                   queue_mark_flush(&mut pending_queue);
                                   continue;
                               }
                               stream_idle_deadline = None;
                               send_text_end_and_await(
                                   &mut write,
                                   &stream_id,
                                   &mut stream_open,
                                   &mut awaiting_terminated,
                                   &mut terminate_deadline,
                               )
                               .await?;
                           }
                           Some(TtsTextCommand::Reset) => {
        // Soft reset: cancel stream, keep authenticated WebSocket.
                               if stream_open || awaiting_terminated {
                                   if let Some(ref id) = stream_id {
                                       let _ = write
                                           .send(Message::Text(build_cancel_message(id)))
                                           .await;
                                   }
                               }
                               pending_queue.clear();
                               stream_open = false;
                               awaiting_terminated = false;
                               terminate_deadline = None;
                               stream_idle_deadline = None;
                               stream_id = None;
                               let _ = flush_coalesce(
                                   &mut coalesce_buf,
                                   audio_tx,
                                   pcm_drops,
                               );
                               idle_deadline = Some(tokio::time::Instant::now() + IDLE_DISCONNECT);
                               debug::log_soniox_tts("soft_reset_keep_ws");
                           }
                           Some(_) => {}
                           None => {
                               if stream_open {
                                   if let Some(ref id) = stream_id {
                                       let _ = write
                                           .send(Message::Text(build_text_message(id, "", true)))
                                           .await;
                                   }
                               }
                               let _ = flush_coalesce(
                                   &mut coalesce_buf,
                                   audio_tx,
                                   pcm_drops,
                               );
                               let _ = write.close().await;
                               return Ok(SessionEnd::Done);
                           }
                       }
                   }
                   maybe_msg = read.next() => {
                       match maybe_msg {
                           Some(Ok(Message::Text(text))) => {
                               let Some(parsed) = parse_tts_message(&text) else {
                                   continue;
                               };
        // A cancelled/terminated stream keeps sending trailing
        // audio/terminated until the server finalizes it. Those
        // messages must not touch the live stream's state.
                               if is_stale_stream_message(
                                   parsed.stream_id.as_deref(),
                                   stream_id.as_deref(),
                               ) {
                                   debug::log_soniox_tts("stale_stream_message_ignored");
                                   continue;
                               }
                               if let Some(err) = parsed.error {
                                   if is_stream_input_closed_error(&err) {
                                       warn!("soniox tts stream input closed; awaiting terminated: {err}");
                                       stream_open = false;
                                       stream_idle_deadline = None;
                                       awaiting_terminated = true;
                                       terminate_deadline =
                                           Some(tokio::time::Instant::now() + TERMINATE_TIMEOUT);
                                       continue;
                                   }
                                   if is_recoverable_tts_error(&err) {
                                       debug::log_soniox_tts("recoverable_error_reconnect");
                                       warn!("soniox tts recoverable error — reconnecting: {err}");
                                       let _ = flush_coalesce(
                                           &mut coalesce_buf,
                                           audio_tx,
                                           pcm_drops,
                                       );
                                       let _ = write.close().await;
                                       if let Some(pending) = pending_work_from_queue(pending_queue) {
                                           return Ok(SessionEnd::Reconnect(pending));
                                       }
                                       return Ok(SessionEnd::Idle);
                                   }
                                   crate::runtime::control_channel::try_send_control(
                                       status_tx,
                                       VoiceTtsStatus::Degraded {
                                           message: format!("Soniox TTS error: {err}"),
                                       },
                                       "tts-status",
                                   );
                                   return Err(anyhow!("soniox tts error: {err}"));
                               }
                               if !parsed.samples.is_empty() {
                                   idle_deadline = None;
                                   let is_first = mark_first_audio_if_unmarked(first_audio_ms);
                                   if is_first {
                                       turn_latency.record_first_audio();
                                       debug::log_soniox_tts("first_audio");
                                   }
                                   coalesce_buf.extend(parsed.samples);
        // Flush first audio immediately for lower TTFB; then coalesce.
                                   if is_first || coalesce_buf.len() >= SONIOX_PCM_COALESCE_MIN_SAMPLES {
                                       flush_coalesce(
                                           &mut coalesce_buf,
                                           audio_tx,
                                           pcm_drops,
                                       )?;
                                   }
                               }
                               if parsed.audio_end || parsed.terminated {
                                   flush_coalesce(
                                       &mut coalesce_buf,
                                       audio_tx,
                                       pcm_drops,
                                   )?;
                               }
                               if parsed.terminated {
                                   stream_open = false;
                                   stream_idle_deadline = None;
                                   awaiting_terminated = false;
                                   terminate_deadline = None;
                                   stream_id = None;
                                   debug::log_soniox_tts("stream_terminated");
                                   debug!("soniox tts stream terminated");
                                   if let Some(next) = pending_queue.pop_front() {
                                       open_and_send_text(
                                           &mut write,
                                           config,
                                           &mut stream_id,
                                           &mut stream_open,
                                           &mut authenticated,
                                           first_audio_ms,
                                           &next.text,
                                       )
                                       .await?;
                                       if next.flush {
                                           send_text_end_and_await(
                                               &mut write,
                                               &stream_id,
                                               &mut stream_open,
                                               &mut awaiting_terminated,
                                               &mut terminate_deadline,
                                           )
                                           .await?;
                                       } else if stream_open {
                                           stream_idle_deadline = Some(
                                               tokio::time::Instant::now() + STREAM_IDLE_TEXT_END,
                                           );
                                       }
                                   } else {
                                       idle_deadline =
                                           Some(tokio::time::Instant::now() + IDLE_DISCONNECT);
                                   }
                               }
                           }
                           Some(Ok(Message::Close(_))) => {
                               let _ = flush_coalesce(
                                   &mut coalesce_buf,
                                   audio_tx,
                                   pcm_drops,
                               );
                               if let Some(pending) = pending_work_from_queue(pending_queue) {
                                   return Ok(SessionEnd::Reconnect(pending));
                               }
                               return Ok(SessionEnd::Idle);
                           }
                           Some(Ok(_)) => {}
                           Some(Err(e)) => {
                               let msg = e.to_string();
                               if is_recoverable_tts_error(&msg) {
                                   debug::log_soniox_tts("ws_error_idle_reconnect");
                                   warn!("soniox tts ws error — idle reconnect: {msg}");
                                   let _ = flush_coalesce(
                                       &mut coalesce_buf,
                                       audio_tx,
                                       pcm_drops,
                                   );
                                   if let Some(pending) = pending_work_from_queue(pending_queue) {
                                       return Ok(SessionEnd::Reconnect(pending));
                                   }
                                   return Ok(SessionEnd::Idle);
                               }
                               return Err(e.into());
                           }
                           None => {
                               let _ = flush_coalesce(
                                   &mut coalesce_buf,
                                   audio_tx,
                                   pcm_drops,
                               );
                               if let Some(pending) = pending_work_from_queue(pending_queue) {
                                   return Ok(SessionEnd::Reconnect(pending));
                               }
                               return Ok(SessionEnd::Idle);
                           }
                       }
                   }
               }
    }
}

fn is_stream_input_closed_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("text_end") && lower.contains("closed for input")
}

/// Returns true once per zeroed slot, stamping it with now. The slot is reset
/// to 0 on every stream open so each turn's first PCM chunk flushes immediately
/// instead of waiting for the coalesce threshold.
fn mark_first_audio_if_unmarked(first_audio_ms: &AtomicU64) -> bool {
    first_audio_ms
        .compare_exchange(
            0,
            crate::audio::monotonic_ms(),
            Ordering::Relaxed,
            Ordering::Relaxed,
        )
        .is_ok()
}

/// Message belongs to a stream we no longer track (e.g. trailing `terminated` /
/// audio of a cancelled stream arriving after its replacement stream opened).
/// Messages without `stream_id` are connection-level and always processed.
fn is_stale_stream_message(msg_stream_id: Option<&str>, current: Option<&str>) -> bool {
    match (msg_stream_id, current) {
        (Some(msg), Some(cur)) => msg != cur,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

fn is_recoverable_tts_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("request timeout")
        || lower.contains("request_timeout")
        || lower.contains("service_unavailable")
        || lower.contains("service unavailable")
        || lower.contains("cannot continue request")
        || lower.contains("timed out")
        || lower.contains("connection reset")
        || lower.contains("broken pipe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_text_end_closed_error() {
        assert!(is_stream_input_closed_error(
            "Stream tts-abc has already received text_end and is closed for input. Start a new stream to send more text."
        ));
        assert!(!is_stream_input_closed_error("invalid api key"));
    }

    #[test]
    fn first_audio_marks_once_per_stream_open() {
        let slot = AtomicU64::new(0);
        assert!(mark_first_audio_if_unmarked(&slot));
        assert!(!mark_first_audio_if_unmarked(&slot));
        // New stream/turn re-arms the immediate first-chunk flush.
        slot.store(0, Ordering::Relaxed);
        assert!(mark_first_audio_if_unmarked(&slot));
        assert!(!mark_first_audio_if_unmarked(&slot));
    }

    #[test]
    fn stale_stream_message_matching() {
        // Same stream → processed.
        assert!(!is_stale_stream_message(Some("tts-a"), Some("tts-a")));
        // Trailing message of a cancelled stream after its replacement opened.
        assert!(is_stale_stream_message(Some("tts-old"), Some("tts-new")));
        // Trailing message after a Reset cleared the current stream id.
        assert!(is_stale_stream_message(Some("tts-old"), None));
        // Connection-level messages (no stream_id) are never stale.
        assert!(!is_stale_stream_message(None, Some("tts-a")));
        assert!(!is_stale_stream_message(None, None));
    }

    #[test]
    fn detects_recoverable_timeout() {
        assert!(is_recoverable_tts_error("Request timeout"));
        assert!(is_recoverable_tts_error("request_timeout"));
        assert!(!is_recoverable_tts_error("Incorrect API key provided"));
    }

    #[test]
    fn pending_queue_keeps_sentence_units_separate() {
        let mut queue = VecDeque::new();
        queue_append_delta(&mut queue, "Sentence one.".into());
        queue_mark_flush(&mut queue);
        queue_append_delta(&mut queue, "Sentence two.".into());
        queue_mark_flush(&mut queue);
        queue_append_delta(&mut queue, "Sentence three.".into());
        queue_mark_flush(&mut queue);

        assert_eq!(queue.len(), 3);
        assert_eq!(queue[0].text, "Sentence one.");
        assert!(queue[0].flush);
        assert_eq!(queue[1].text, "Sentence two.");
        assert!(queue[1].flush);
        assert_eq!(queue[2].text, "Sentence three.");
        assert!(queue[2].flush);
    }

    #[test]
    fn pending_queue_appends_into_unflushed_unit() {
        let mut queue = VecDeque::new();
        queue_append_delta(&mut queue, "Hello".into());
        queue_append_delta(&mut queue, " world".into());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].text, "Hello world");
        assert!(!queue[0].flush);

        queue_mark_flush(&mut queue);
        queue_append_delta(&mut queue, "Next".into());
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[1].text, "Next");
        assert!(!queue[1].flush);
    }

    #[test]
    fn coalesce_matches_elevenlabs() {
        assert_eq!(
            SONIOX_PCM_COALESCE_MIN_SAMPLES,
            crate::voice::config::EL_PCM_COALESCE_MIN_SAMPLES
        );
    }
}
