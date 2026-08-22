use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, tungstenite::Message,
};
use tracing::info;

use super::protocol::{
    build_audio_append, build_session_close, build_session_update,
    build_transcription_audio_append, build_transcription_audio_commit,
    build_transcription_session_update, parse_api_error, parse_server_event, PhraseCommitter,
    PHRASE_SILENCE_COMMIT_MS,
};
use crate::audio::resampler::resample_for_provider_upload;
use crate::audio::try_send_pcm_bounded;
use crate::providers::openai::config::{
    transcription_ws_url, translations_ws_url, UPLOAD_SAMPLE_RATE,
};
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
pub struct OpenAiBridgeHandle {
    core: BridgeWorkerCore,
}

impl OpenAiBridgeHandle {
    pub async fn connect(
        api_key: &str,
        target_language: &str,
        direction: &str,
        live_model: &str,
        translation_enabled: bool,
        play_audio: Arc<AtomicBool>,
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

        let api_key = api_key.to_string();
        let target_language = target_language.to_string();
        let direction_label = direction.to_string();
        let target_language_log = target_language.clone();
        let live_model = live_model.to_string();

        let play_audio_loop = play_audio.clone();
        let join = tokio::spawn(async move {
            let config = ReconnectLoopConfig {
                provider_label: "OpenAI",
                direction: &direction_label,
                reconnect_policy,
            };
            let mut state = ReconnectState::new();

            loop {
                if child_cancel.is_cancelled() {
                    break;
                }

                match run_single_session(
                    &api_key,
                    &target_language,
                    &direction_label,
                    &live_model,
                    translation_enabled,
                    play_audio_loop.clone(),
                    &mut audio_in_rx,
                    &audio_out_tx,
                    &pcm_drops,
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

        let timeout_hint = if translation_enabled {
            format!("OpenAI setup timeout for {direction}. Check API key and translation access.")
        } else {
            format!(
                "OpenAI Notes setup timeout for {direction}. Check API key and Realtime transcription access."
            )
        };
        core.wait_until_ready("OpenAI", direction, &timeout_hint)
            .await?;

        info!(
            "[OpenAI:{direction}] bridge ready -> {target_language_log} (translation={translation_enabled})"
        );
        Ok(Self { core })
    }

    pub fn send_audio(&self, pcm: &[i16]) {
        let upload = resample_for_provider_upload(pcm, UPLOAD_SAMPLE_RATE);
        self.core.try_send_audio(upload);
    }

    pub fn is_ready(&self) -> bool {
        self.core.is_ready()
    }

    pub fn ready_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.core.ready_flag()
    }

    pub async fn stop(self) {
        self.core.stop().await;
    }

    pub async fn abort(self) {
        self.core.abort().await;
    }
}

fn noise_reduction_for_direction(direction: &str) -> &'static str {
    if direction == "inbound" {
        "far_field"
    } else {
        "near_field"
    }
}

async fn run_single_session(
    api_key: &str,
    target_language: &str,
    direction: &str,
    live_model: &str,
    translation_enabled: bool,
    play_audio: Arc<AtomicBool>,
    audio_in_rx: &mut tokio::sync::mpsc::Receiver<Vec<i16>>,
    audio_out_tx: &tokio::sync::mpsc::Sender<Vec<i16>>,
    pcm_drops: &std::sync::atomic::AtomicU64,
    transcript_tx: &TranscriptSender,
    setup_complete: &Arc<std::sync::atomic::AtomicBool>,
    cancel: &tokio_util::sync::CancellationToken,
    status_tx: &BridgeStatusSender,
    reconnected: bool,
) -> Result<()> {
    let url = if translation_enabled {
        translations_ws_url(live_model)
    } else {
        transcription_ws_url()
    };
    let mut request = url
        .into_client_request()
        .context("build OpenAI WebSocket request")?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {api_key}")
            .parse()
            .context("invalid Authorization header")?,
    );

    let (ws_stream, _) =
        tokio::time::timeout(tokio::time::Duration::from_secs(15), connect_async(request))
            .await
            .map_err(|_| SessionError::retryable("OpenAI WebSocket connect timeout"))?
            .context("failed to connect OpenAI WebSocket")?;

    let (mut write, mut read) = ws_stream.split();

    let noise = noise_reduction_for_direction(direction);
    let setup_msg = if translation_enabled {
        build_session_update(target_language, noise)
    } else {
        build_transcription_session_update(target_language, noise)
    };
    write
        .send(Message::Text(setup_msg))
        .await
        .context("failed to send session.update")?;

    setup_complete.store(false, std::sync::atomic::Ordering::SeqCst);
    let mut ready_emitted = false;
    let mut committer = PhraseCommitter::default();
    let mut closing = false;
    let mut pending_audio_since: Option<Instant> = None;
    let mut silence_tick = tokio::time::interval(tokio::time::Duration::from_millis(100));
    silence_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                closing = true;
                if !translation_enabled && pending_audio_since.is_some() {
                    let _ = write.send(Message::Text(build_transcription_audio_commit())).await;
                    pending_audio_since = None;
                }
                let _ = write.send(Message::Text(build_session_close())).await;
            }
            _ = silence_tick.tick() => {
                if !translation_enabled {
                    if let Some(since) = pending_audio_since {
                        if since.elapsed()
                            >= std::time::Duration::from_millis(PHRASE_SILENCE_COMMIT_MS)
                            && setup_complete.load(Ordering::SeqCst)
                            && !closing
                        {
                            let _ = write
                                .send(Message::Text(build_transcription_audio_commit()))
                                .await;
                            pending_audio_since = None;
                        }
                    }
                } else {
                    for t in committer.check_paired_done(direction) {
                        crate::runtime::control_channel::try_send_control(
                            transcript_tx,
                            t,
                            "transcript-fanout",
                        );
                    }
                    for t in committer.check_silence(direction) {
                        crate::runtime::control_channel::try_send_control(
                            transcript_tx,
                            t,
                            "transcript-fanout",
                        );
                    }
                }
            }
            maybe_audio = audio_in_rx.recv() => {
                match maybe_audio {
                    Some(pcm) if setup_complete.load(Ordering::SeqCst) && !closing => {
                        let msg = if translation_enabled {
                            build_audio_append(&pcm)
                        } else {
                            build_transcription_audio_append(&pcm)
                        };
                        write.send(Message::Text(msg)).await?;
                        if !translation_enabled {
                            pending_audio_since = Some(Instant::now());
                        }
                    }
                    Some(_) => {}
                    None => {
                        if !closing {
                            closing = true;
                            if !translation_enabled && pending_audio_since.is_some() {
                                let _ = write
                                    .send(Message::Text(build_transcription_audio_commit()))
                                    .await;
                                pending_audio_since = None;
                            }
                            let _ = write.send(Message::Text(build_session_close())).await;
                        }
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
                            return Err(SessionError::retryable("OpenAI WebSocket closed"));
                        }

                        let Some(text) = decode_ws_message(msg) else {
                            continue;
                        };

                        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                            continue;
                        };

                        if let Some(err) = parse_api_error(&value) {
                            return Err(SessionError::fatal(format!("OpenAI API error: {err}")));
                        }

                        let (setup, chunks, transcripts, session_closed) =
                            parse_server_event(&value, direction, &mut committer);

                        if setup {
                            setup_complete.store(true, Ordering::SeqCst);
                            info!("[OpenAI:{direction}] session ready");
                            if !ready_emitted {
                                ready_emitted = true;
                                crate::runtime::control_channel::try_send_control(
                                    status_tx,
                                    BridgeStatusEvent::Ready {
                                        direction: direction.to_string(),
                                        reconnected,
                                    },
                                    "bridge-status",
                                );
                            }
                        }

                        if translation_enabled && play_audio.load(Ordering::SeqCst) {
                            for chunk in chunks {
                                let _ = try_send_pcm_bounded(
                                    audio_out_tx,
                                    chunk.pcm_24k,
                                    pcm_drops,
                                );
                            }
                        }
                        for t in transcripts {
                            crate::runtime::control_channel::try_send_control(
                            transcript_tx,
                            t,
                            "transcript-fanout",
                        );
                        }

                        if session_closed {
                            break;
                        }
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => {
                        if closing {
                            break;
                        }
                        return Err(SessionError::retryable("OpenAI WebSocket stream ended"));
                    }
                }
            }
        }
    }

    Ok(())
}
