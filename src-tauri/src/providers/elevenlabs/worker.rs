use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::audio::try_send_pcm_bounded;
use crate::audio::PlaybackPcmChunk;
use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, tungstenite::Message,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::config::stream_input_url;
use super::latency::TurnLatencySlot;
use super::protocol::{
    build_close_message, build_flush_message, build_init_message, build_text_chunk,
    is_non_retryable_elevenlabs_error, parse_audio_message, parse_error_message,
    user_message_for_elevenlabs_error, ElevenLabsInitSettings,
};
use crate::voice::shared::debug;
use crate::voice::shared::types::VoiceTtsStatus;

pub use crate::voice::shared::tts_command::TtsTextCommand;

#[derive(Debug, Clone)]
pub struct ElevenLabsWorkerConfig {
    pub api_key: String,
    pub voice_id: String,
    pub model_id: String,
    pub init_settings: ElevenLabsInitSettings,
    pub language_code: Option<String>,
    pub auto_mode: bool,
}

enum SessionEnd {
    Done,
    Reset,
}

pub fn spawn_elevenlabs_tts_worker(
    config: ElevenLabsWorkerConfig,
    cmd_rx: mpsc::Receiver<TtsTextCommand>,
    audio_tx: mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: Arc<AtomicU64>,
    status_tx: mpsc::Sender<VoiceTtsStatus>,
    turn_latency: Arc<TurnLatencySlot>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_elevenlabs_tts_worker(
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
            warn!("elevenlabs tts worker exited: {e:#}");
        }
    })
}

async fn run_elevenlabs_tts_worker(
    config: ElevenLabsWorkerConfig,
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
                if is_non_retryable_elevenlabs_error(&raw) {
                    let message = user_message_for_elevenlabs_error(&raw);
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                attempts += 1;
                if attempts >= 3 {
                    let message = user_message_for_elevenlabs_error(&raw);
                    crate::runtime::control_channel::try_send_control(
                        &status_tx,
                        VoiceTtsStatus::Degraded { message },
                        "tts-status",
                    );
                    return Err(err);
                }
                warn!("elevenlabs session error (attempt {attempts}): {err:#}");
                let backoff = Duration::from_secs(1u64 << attempts.saturating_sub(1).min(2));
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }
            }
        }
    }
    info!("elevenlabs tts worker shut down cleanly");
    Ok(())
}

async fn run_session(
    config: &ElevenLabsWorkerConfig,
    cmd_rx: &mut mpsc::Receiver<TtsTextCommand>,
    audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
    pcm_drops: &AtomicU64,
    status_tx: &mpsc::Sender<VoiceTtsStatus>,
    turn_latency: &TurnLatencySlot,
    first_audio_ms: &Arc<AtomicU64>,
    cancel: &CancellationToken,
    reconnected: bool,
) -> Result<SessionEnd> {
    let url = stream_input_url(
        &config.voice_id,
        &config.model_id,
        config.language_code.as_deref(),
        config.auto_mode,
    );
    let mut request = url.into_client_request().context("elevenlabs ws request")?;
    request.headers_mut().insert(
        "xi-api-key",
        config.api_key.parse().context("api key header")?,
    );

    let (ws, _) = tokio::time::timeout(Duration::from_secs(15), connect_async(request))
        .await
        .map_err(|_| anyhow!("elevenlabs connect timeout"))?
        .map_err(|e| anyhow!("elevenlabs connect failed: {e}"))?;

    let (mut write, mut read) = ws.split();

    write
        .send(Message::Text(build_init_message(&config.init_settings)))
        .await
        .context("elevenlabs init send")?;
    debug::log_elevenlabs_ws_init(config.init_settings.chunk_schedule, config.auto_mode);
    crate::runtime::control_channel::try_send_control(
        status_tx,
        VoiceTtsStatus::Ready,
        "tts-status",
    );
    if reconnected {
        debug!("elevenlabs tts websocket reconnected");
    } else {
        info!("elevenlabs tts websocket ready");
    }

    let keepalive_interval = Duration::from_secs(45);
    let mut keepalive_at = tokio::time::Instant::now() + keepalive_interval;
    let mut coalesce_buf: Vec<i16> = Vec::new();

    let flush_coalesce = |buf: &mut Vec<i16>,
                          audio_tx: &mpsc::Sender<PlaybackPcmChunk>,
                          pcm_drops: &AtomicU64,
                          status_tx: &mpsc::Sender<VoiceTtsStatus>|
     -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let chunk = PlaybackPcmChunk::new(std::mem::take(buf));
        if !try_send_pcm_bounded(audio_tx, chunk, pcm_drops) && audio_tx.is_closed() {
            warn!("elevenlabs audio playback channel closed");
            let message = "Custom voice playback unavailable — Stop and Start to retry.".into();
            crate::runtime::control_channel::try_send_control(
                status_tx,
                VoiceTtsStatus::Degraded { message },
                "tts-status",
            );
            return Err(anyhow!("playback channel closed"));
        }
        Ok(())
    };

    let mut deliver_pcm = |parsed: super::protocol::ParsedAudio| -> Result<()> {
        if parsed.is_final {
            debug::log_elevenlabs_ws_is_final();
        }

        if !parsed.samples.is_empty() {
            debug!(samples = parsed.samples.len(), "elevenlabs received audio");
            if first_audio_ms.load(Ordering::Relaxed) == 0 {
                let now = crate::audio::monotonic_ms();
                first_audio_ms.store(now, Ordering::Relaxed);
                turn_latency.record_first_audio();
            }
            coalesce_buf.extend(parsed.samples);
        }

        if parsed.is_final {
            return flush_coalesce(&mut coalesce_buf, audio_tx, pcm_drops, status_tx);
        }
        if coalesce_buf.len() >= crate::voice::config::EL_PCM_COALESCE_MIN_SAMPLES {
            return flush_coalesce(&mut coalesce_buf, audio_tx, pcm_drops, status_tx);
        }
        Ok(())
    };

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = flush_coalesce(&mut coalesce_buf, audio_tx, pcm_drops, status_tx);
                let _ = write.send(Message::Text(build_close_message())).await;
                let _ = write.close().await;
                info!("elevenlabs tts websocket closed on cancel");
                return Ok(SessionEnd::Done);
            }
            cmd = cmd_rx.recv() => {
                keepalive_at = tokio::time::Instant::now() + keepalive_interval;
                match cmd {
                    Some(TtsTextCommand::AppendDelta {
                        text,
                        trigger_generation,
                    }) if !text.is_empty() => {
                        debug::log_elevenlabs_ws_text(&text, trigger_generation);
                        debug!(
                            chars = text.chars().count(),
                            trigger = trigger_generation,
                            "elevenlabs send text chunk"
                        );
                        write
                            .send(Message::Text(
                                build_text_chunk(&text, trigger_generation),
                            ))
                            .await
                            .context("elevenlabs text send")?;
                    }
                    Some(TtsTextCommand::Flush) => {
                        debug::log_elevenlabs_ws_flush();
                        debug!("elevenlabs send flush");
                        write
                            .send(Message::Text(build_flush_message()))
                            .await
                            .context("elevenlabs flush send")?;
                    }
                    Some(TtsTextCommand::Reset) => {
                        let _ = flush_coalesce(
                            &mut coalesce_buf,
                            audio_tx,
                            pcm_drops,
                    status_tx,
                        );
                        debug!("elevenlabs reset session");
                        let _ = write.send(Message::Text(build_close_message())).await;
                        let _ = write.close().await;
                        return Ok(SessionEnd::Reset);
                    }
                    None => {
                        let _ = flush_coalesce(
                            &mut coalesce_buf,
                            audio_tx,
                            pcm_drops,
                    status_tx,
                        );
                        return Ok(SessionEnd::Done);
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep_until(keepalive_at) => {
                keepalive_at = tokio::time::Instant::now() + keepalive_interval;
                write
                    .send(Message::Text(build_text_chunk(" ", false)))
                    .await
                    .context("elevenlabs keepalive send")?;
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(parsed) = parse_audio_message(&text) {
                            if deliver_pcm(parsed).is_err() {
                                return Ok(SessionEnd::Done);
                            }
                        } else if let Some(err) = parse_error_message(&text) {
                            warn!("elevenlabs server error: {err}");
                            return Err(anyhow!("elevenlabs server error: {err}"));
                        }
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        if let Ok(text) = String::from_utf8(bytes) {
                            if let Some(parsed) = parse_audio_message(&text) {
                                if deliver_pcm(parsed).is_err() {
                                    return Ok(SessionEnd::Done);
                                }
                            } else if let Some(err) = parse_error_message(&text) {
                                warn!("elevenlabs server error: {err}");
                                return Err(anyhow!("elevenlabs server error: {err}"));
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write
                            .send(Message::Pong(data))
                            .await
                            .context("elevenlabs pong send")?;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        return Err(anyhow!(
                            "elevenlabs closed connection: {}",
                            frame.map(|f| f.reason.to_string()).unwrap_or_default()
                        ));
                    }
                    Some(Ok(Message::Frame(_))) => {}
                    None => {
                        return Err(anyhow!("elevenlabs websocket stream ended"));
                    }
                    Some(Err(e)) => {
                        return Err(anyhow!("elevenlabs read error: {e}"));
                    }
                }
            }
        }
    }
}
