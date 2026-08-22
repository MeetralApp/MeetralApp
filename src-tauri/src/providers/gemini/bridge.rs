use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, tungstenite::Message,
};
use tracing::info;

use super::protocol::{
    build_audio_message, build_setup_message, parse_api_error, parse_server_message_from_value,
};
use crate::audio::resampler::resample_for_provider_upload;
use crate::audio::try_send_pcm_bounded;
use crate::providers::gemini::config::{GOOGLE_API_KEY_HEADER, UPLOAD_SAMPLE_RATE};
use crate::providers::shared::live::reconnect::{
    apply_retry, classify_session_error, ReconnectAction, ReconnectLoopConfig, ReconnectState,
};
use crate::providers::shared::live::session_error::SessionError;
use crate::providers::shared::live::shared::{
    decode_ws_message, BridgeFatalSender, BridgeStatusEvent, BridgeStatusSender, LiveSetupOptions,
    ReconnectPolicy, TranscriptSender,
};
use crate::providers::shared::live::worker::BridgeWorkerCore;

#[derive(Clone)]
pub struct GeminiBridgeHandle {
    core: BridgeWorkerCore,
}

impl GeminiBridgeHandle {
    pub async fn connect(
        api_key: &str,
        target_language: &str,
        direction: &str,
        setup_options: LiveSetupOptions,
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

        let play_audio_loop = play_audio.clone();
        let join = tokio::spawn(async move {
            let config = ReconnectLoopConfig {
                provider_label: "Gemini",
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
                    &setup_options,
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

        core.wait_until_ready(
            "Gemini",
            direction,
            &format!("Gemini setup timeout for {direction}. Check API key and Live API access."),
        )
        .await?;

        info!("[Gemini:{direction}] bridge ready -> {target_language_log}");
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

async fn run_single_session(
    api_key: &str,
    target_language: &str,
    direction: &str,
    setup_options: &LiveSetupOptions,
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
    let url = super::protocol::ws_url();
    let mut request = url
        .into_client_request()
        .context("build Gemini WebSocket request")?;
    request.headers_mut().insert(
        tokio_tungstenite::tungstenite::http::header::HeaderName::from_static(
            GOOGLE_API_KEY_HEADER,
        ),
        api_key.parse().context("invalid x-goog-api-key header")?,
    );
    let (ws_stream, _) =
        tokio::time::timeout(tokio::time::Duration::from_secs(15), connect_async(request))
            .await
            .map_err(|_| SessionError::retryable("Gemini WebSocket connect timeout"))?
            .context("failed to connect Gemini WebSocket")?;

    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(build_setup_message(
            target_language,
            setup_options,
        )))
        .await
        .context("failed to send setup")?;

    setup_complete.store(false, std::sync::atomic::Ordering::SeqCst);
    let mut ready_emitted = false;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = write.send(Message::Close(None)).await;
                break;
            }
            maybe_audio = audio_in_rx.recv() => {
                match maybe_audio {
                    Some(pcm) if setup_complete.load(std::sync::atomic::Ordering::SeqCst) => {
                        let msg = build_audio_message(&pcm, UPLOAD_SAMPLE_RATE);
                        write.send(Message::Text(msg)).await?;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
            maybe_msg = read.next() => {
                match maybe_msg {
                    Some(Ok(msg)) => {
                        if matches!(&msg, Message::Close(_)) {
                            return Err(SessionError::retryable("Gemini WebSocket closed"));
                        }

                        let Some(text) = decode_ws_message(msg) else {
                            continue;
                        };

                        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                            continue;
                        };

                        if let Some(err) = parse_api_error(&value) {
                            return Err(SessionError::fatal(format!("Gemini API error: {err}")));
                        }

                        let (setup, chunks, transcripts, _) =
                            parse_server_message_from_value(&value, direction);

                        if setup {
                            setup_complete.store(true, std::sync::atomic::Ordering::SeqCst);
                            info!("[Gemini:{direction}] setup complete");
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

                        if play_audio.load(Ordering::SeqCst) {
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
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => return Err(SessionError::retryable("Gemini WebSocket stream ended")),
                }
            }
        }
    }

    Ok(())
}
