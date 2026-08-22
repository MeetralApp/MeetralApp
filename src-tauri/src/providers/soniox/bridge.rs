use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::info;

use super::config::stt_ws_url;
use super::config::UPLOAD_SAMPLE_RATE;
use super::context::SonioxContextInput;
use super::debug;
use super::protocol::{
    build_stt_config_message, build_stt_keepalive_message, parse_api_error, parse_stt_message,
    response_finished, SonioxSttSetup, SonioxTokenAccumulator,
};
use crate::audio::resampler::resample_for_provider_upload;
use crate::providers::shared::live::reconnect::{
    apply_retry, classify_session_error, ReconnectAction, ReconnectLoopConfig, ReconnectState,
};
use crate::providers::shared::live::session_error::SessionError;
use crate::providers::shared::live::shared::{
    decode_ws_message, BridgeFatalSender, BridgeStatusEvent, BridgeStatusSender, ReconnectPolicy,
    TranscriptSender,
};
use crate::providers::shared::live::worker::BridgeWorkerCore;

#[derive(Clone)]
pub struct SonioxBridgeHandle {
    core: BridgeWorkerCore,
}

impl SonioxBridgeHandle {
    pub async fn connect(
        api_key: &str,
        target_language: &str,
        direction: &str,
        live_model: &str,
        language_hints: Vec<String>,
        context: SonioxContextInput,
        endpoint_latency_adjustment_level: u8,
        endpoint_sensitivity: f64,
        max_endpoint_delay_ms: u32,
        translation_enabled: bool,
        _play_audio: Arc<AtomicBool>,
        // Soniox STT has no STS audio, but outbound mux watches this channel.
        // Dropping it early closes bridge_pcm_rx and tears down the mux (and TTS PCM).
        audio_out_tx: tokio::sync::mpsc::Sender<Vec<i16>>,
        pcm_drops: Arc<std::sync::atomic::AtomicU64>,
        transcript_tx: TranscriptSender,
        fatal_tx: BridgeFatalSender,
        status_tx: BridgeStatusSender,
        reconnect_policy: ReconnectPolicy,
    ) -> Result<Self> {
        let (core, mut audio_in_rx) = BridgeWorkerCore::open(pcm_drops.clone());
        let child_cancel = core.child_cancel();
        let setup_flag = core.setup_flag();

        let setup = SonioxSttSetup {
            api_key: api_key.to_string(),
            model: live_model.to_string(),
            target_language: target_language.to_string(),
            language_hints,
            context,
            endpoint_latency_adjustment_level,
            endpoint_sensitivity,
            max_endpoint_delay_ms,
            translation_enabled,
        };
        let direction_label = direction.to_string();
        let target_language_log = target_language.to_string();

        let join = tokio::spawn(async move {
            // Keep sender alive for the worker lifetime so outbound mux stays up.
            let _audio_out_keepalive = audio_out_tx;

            let config = ReconnectLoopConfig {
                provider_label: "Soniox",
                direction: &direction_label,
                reconnect_policy,
            };
            let mut state = ReconnectState::new();

            loop {
                if child_cancel.is_cancelled() {
                    break;
                }

                match run_single_session(
                    &setup,
                    &direction_label,
                    &mut audio_in_rx,
                    &transcript_tx,
                    &setup_flag,
                    &child_cancel,
                    &status_tx,
                    state.reconnected(),
                )
                .await
                {
                    Ok(()) => break,
                    Err(err) => match classify_session_error(err, &config, &mut state) {
                        ReconnectAction::BreakFatal(message) => {
                            crate::runtime::control_channel::try_send_control(
                                &fatal_tx,
                                message,
                                "bridge-fatal",
                            );
                            break;
                        }
                        ReconnectAction::Retry { delay } => {
                            apply_retry(
                                &setup_flag,
                                &status_tx,
                                &direction_label,
                                state.attempts,
                                delay,
                            )
                            .await;
                        }
                    },
                }
            }
        });

        core.attach_worker(join).await;

        core.wait_until_ready(
            "Soniox",
            direction,
            &format!("Soniox setup timeout for {direction}. Check API key and STT access."),
        )
        .await?;

        info!("[Soniox:{direction}] bridge ready -> {target_language_log}");
        Ok(Self { core })
    }

    pub fn send_audio(&self, pcm: &[i16]) {
        let upload = resample_for_provider_upload(pcm, UPLOAD_SAMPLE_RATE);
        self.core.try_send_audio(upload);
    }

    pub fn is_ready(&self) -> bool {
        self.core.is_ready()
    }

    pub fn ready_flag(&self) -> Arc<AtomicBool> {
        self.core.ready_flag()
    }

    pub async fn stop(self) {
        self.core.stop().await;
    }

    pub async fn abort(self) {
        self.core.abort().await;
    }
}

async fn run_single_session(
    setup: &SonioxSttSetup,
    direction: &str,
    audio_in_rx: &mut tokio::sync::mpsc::Receiver<Vec<i16>>,
    transcript_tx: &TranscriptSender,
    setup_complete: &Arc<AtomicBool>,
    cancel: &tokio_util::sync::CancellationToken,
    status_tx: &BridgeStatusSender,
    reconnected: bool,
) -> Result<()> {
    // Reset on every reconnect attempt (Soniox has no setupComplete event).
    setup_complete.store(false, Ordering::SeqCst);

    let url = stt_ws_url();
    let (ws_stream, _) =
        tokio::time::timeout(tokio::time::Duration::from_secs(15), connect_async(url))
            .await
            .map_err(|_| SessionError::retryable("Soniox WebSocket connect timeout"))?
            .context("failed to connect Soniox STT WebSocket")?;

    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(build_stt_config_message(setup)))
        .await
        .context("failed to send Soniox STT config")?;

    // Soniox has no setupComplete. Wait for first server JSON (or grace window)
    // before Ready — matches commands.rs test_api_key behavior.
    const READY_GRACE: tokio::time::Duration = tokio::time::Duration::from_millis(400);
    let ready_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(15);
    let mut marked_ready = false;
    let mut pending_first_text: Option<String> = None;

    while !marked_ready {
        if cancel.is_cancelled() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= ready_deadline {
            return Err(SessionError::retryable(format!(
                "Soniox setup timeout for {direction}. Check API key and STT access."
            )));
        }
        tokio::select! {
                   _ = cancel.cancelled() => return Ok(()),
                   maybe_msg = read.next() => {
                       match maybe_msg {
                           Some(Ok(msg)) => {
                               if matches!(&msg, Message::Close(_)) {
                                   return Err(SessionError::retryable(
                                       "Soniox WebSocket closed before ready",
                                   ));
                               }
                               let Some(text) = decode_ws_message(msg) else {
                                   continue;
                               };
                               if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                   if let Some(err) = parse_api_error(&value) {
                                       return Err(SessionError::fatal(format!("Soniox API error: {err}")));
                                   }
                                   if value.get("error_type").is_some() {
                                       let msg = value
                                           .get("error_message")
                                           .and_then(|m| m.as_str())
                                           .unwrap_or("unknown");
                                       return Err(SessionError::fatal(format!("Soniox API error: {msg}")));
                                   }
        // First non-error JSON → config accepted; keep text for token parse.
                                   pending_first_text = Some(text);
                                   marked_ready = true;
                               }
                           }
                           Some(Err(e)) => return Err(e.into()),
                           None => {
                               return Err(SessionError::retryable(
                                   "Soniox WebSocket stream ended before ready",
                               ))
                           }
                       }
                   }
                   _ = tokio::time::sleep(READY_GRACE) => {
        // No error after config within grace → treat as ready (same as key test).
                       marked_ready = true;
                   }
               }
    }

    setup_complete.store(true, Ordering::SeqCst);
    info!("[Soniox:{direction}] session ready");
    crate::runtime::control_channel::try_send_control(
        status_tx,
        BridgeStatusEvent::Ready {
            direction: direction.to_string(),
            reconnected,
        },
        "bridge-status",
    );

    let mut acc = SonioxTokenAccumulator::default();
    let mut closing = false;
    let mut first_token_logged = false;
    // Docs: send keepalive at least every 20s when not sending audio.
    const KEEPALIVE_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_secs(15);
    /// Cap wait for final STT tokens after empty-string close. Must stay under
    /// `BridgeWorkerCore` stop join timeout so user-initiated stop stays snappy.
    const FINISHED_WAIT: tokio::time::Duration = tokio::time::Duration::from_millis(500);
    let mut keepalive_at = tokio::time::Instant::now() + KEEPALIVE_INTERVAL;
    let mut finished_deadline: Option<tokio::time::Instant> = None;

    if let Some(text) = pending_first_text {
        dispatch_stt_text(
            &text,
            direction,
            &mut acc,
            transcript_tx,
            &mut first_token_logged,
        );
    }

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled(), if !closing => {
                closing = true;
                let _ = write.send(Message::Text("".into())).await;
                finished_deadline = Some(tokio::time::Instant::now() + FINISHED_WAIT);
            }
            _ = async {
                match finished_deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            }, if closing && finished_deadline.is_some() => {
                info!("[Soniox:{direction}] finished wait timeout — closing");
                debug::log_soniox_stt(direction, "finished_wait_timeout");
                let _ = write.close().await;
                break;
            }
            _ = tokio::time::sleep_until(keepalive_at), if !closing => {
                keepalive_at = tokio::time::Instant::now() + KEEPALIVE_INTERVAL;
                let _ = write
                    .send(Message::Text(build_stt_keepalive_message()))
                    .await;
            }
            maybe_audio = audio_in_rx.recv(), if !closing => {
                match maybe_audio {
                    Some(pcm) if setup_complete.load(Ordering::SeqCst) => {
                        keepalive_at = tokio::time::Instant::now() + KEEPALIVE_INTERVAL;
                        let bytes: Vec<u8> = pcm
                            .iter()
                            .flat_map(|s| s.to_le_bytes())
                            .collect();
                        write.send(Message::Binary(bytes)).await?;
                    }
                    Some(_) => {}
                    None => {
                        closing = true;
                        let _ = write.send(Message::Text("".into())).await;
                        finished_deadline = Some(tokio::time::Instant::now() + FINISHED_WAIT);
                    }
                }
            }
            maybe_msg = read.next() => {
                match maybe_msg {
                    Some(Ok(msg)) => {
                        if matches!(&msg, Message::Close(_)) {
                            if closing {
                                break;
                            }
                            return Err(SessionError::retryable("Soniox WebSocket closed"));
                        }

                        let Some(text) = decode_ws_message(msg) else {
                            continue;
                        };

                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(err) = parse_api_error(&value) {
                                return Err(SessionError::fatal(format!("Soniox API error: {err}")));
                            }
                            if value.get("error_type").is_some() {
                                let msg = value
                                    .get("error_message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("unknown");
                                return Err(SessionError::fatal(format!("Soniox API error: {msg}")));
                            }
                            if response_finished(&value) {
                                debug::log_soniox_stt(direction, "finished");
                                dispatch_stt_text(
                                    &text,
                                    direction,
                                    &mut acc,
                                    transcript_tx,
                                    &mut first_token_logged,
                                );
                                let _ = write.close().await;
                                break;
                            }
                        }

                        dispatch_stt_text(
                            &text,
                            direction,
                            &mut acc,
                            transcript_tx,
                            &mut first_token_logged,
                        );
                    }
                    Some(Err(e)) => {
                        if closing {
                            break;
                        }
                        return Err(e.into());
                    }
                    None => {
                        if closing {
                            break;
                        }
                        return Err(SessionError::retryable("Soniox WebSocket stream ended"));
                    }
                }
            }
        }
    }

    Ok(())
}

fn dispatch_stt_text(
    text: &str,
    direction: &str,
    acc: &mut SonioxTokenAccumulator,
    transcript_tx: &TranscriptSender,
    first_token_logged: &mut bool,
) {
    let events = parse_stt_message(text, direction, acc);
    for t in events {
        if !*first_token_logged && (t.translated_text.is_some() || t.source_text.is_some()) {
            *first_token_logged = true;
            debug::log_soniox_stt(direction, "first_token");
        }
        if t.turn_complete {
            let source_chars = t
                .source_text
                .as_deref()
                .map(|s| s.chars().count())
                .unwrap_or(0);
            let translated_chars = t
                .translated_text
                .as_deref()
                .map(|s| s.chars().count())
                .unwrap_or(0);
            debug::log_soniox_endpoint(direction, source_chars, translated_chars);
        }
        crate::runtime::control_channel::try_send_control(transcript_tx, t, "transcript-fanout");
    }
}
