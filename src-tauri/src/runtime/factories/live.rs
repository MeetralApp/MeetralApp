//! Live bridge factory — the only allowed `match AiProvider` for connect / API key test.

use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use tokio::sync::mpsc;

use crate::ai::AiProvider;
use crate::providers::shared::live::{
    BridgeFatalSender, BridgeStatusSender, LiveBridgeHandle, LiveSetupOptions, ReconnectPolicy,
    TranscriptSender,
};
use crate::providers::soniox::context::SonioxContextInput;
use crate::providers::{gemini, openai, soniox};

/// Connect the live STT/translate bridge for `provider`.
///
/// Session code must call this instead of matching on `AiProvider` itself.
pub async fn connect_live_bridge_for(
    provider: AiProvider,
    api_key: &str,
    target_language: &str,
    direction: &str,
    setup_options: LiveSetupOptions,
    play_audio: Arc<AtomicBool>,
    audio_out_tx: mpsc::Sender<Vec<i16>>,
    pcm_drops: Arc<std::sync::atomic::AtomicU64>,
    transcript_tx: TranscriptSender,
    fatal_tx: BridgeFatalSender,
    status_tx: BridgeStatusSender,
    reconnect_policy: ReconnectPolicy,
) -> Result<LiveBridgeHandle> {
    match provider {
        AiProvider::Gemini => {
            let bridge = gemini::GeminiBridgeHandle::connect(
                api_key,
                target_language,
                direction,
                setup_options,
                play_audio,
                audio_out_tx,
                pcm_drops,
                transcript_tx,
                fatal_tx,
                status_tx,
                reconnect_policy,
            )
            .await?;
            Ok(LiveBridgeHandle::Gemini(bridge))
        }
        AiProvider::OpenAi => {
            let bridge = openai::OpenAiBridgeHandle::connect(
                api_key,
                target_language,
                direction,
                &setup_options.model,
                setup_options.translation_enabled,
                play_audio,
                audio_out_tx,
                pcm_drops,
                transcript_tx,
                fatal_tx,
                status_tx,
                reconnect_policy,
            )
            .await?;
            Ok(LiveBridgeHandle::OpenAi(bridge))
        }
        AiProvider::Soniox => {
            let context = SonioxContextInput {
                general: setup_options.soniox_general.clone(),
                text: setup_options.soniox_context_text.clone(),
                terms: setup_options.soniox_glossary_terms.clone(),
                translation_terms: setup_options
                    .soniox_translation_terms
                    .iter()
                    .map(|t| (t.source.clone(), t.target.clone()))
                    .collect(),
            };
            let bridge = soniox::SonioxBridgeHandle::connect(
                api_key,
                target_language,
                direction,
                &setup_options.model,
                setup_options.language_hints.clone(),
                context,
                setup_options.soniox_endpoint_latency_adjustment_level,
                setup_options.soniox_endpoint_sensitivity,
                setup_options.soniox_max_endpoint_delay_ms,
                setup_options.translation_enabled,
                play_audio,
                audio_out_tx,
                pcm_drops,
                transcript_tx,
                fatal_tx,
                status_tx,
                reconnect_policy,
            )
            .await?;
            Ok(LiveBridgeHandle::Soniox(bridge))
        }
    }
}

/// Validate a live-provider API key with a short WebSocket handshake.
pub async fn test_live_api_key(
    provider: AiProvider,
    api_key: &str,
    setup_options: &LiveSetupOptions,
) -> Result<(), String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    if api_key.trim().is_empty() {
        return Err("API key is empty".into());
    }

    match provider {
        AiProvider::Gemini => {
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;

            let url = gemini::protocol::ws_url();
            let mut request = url
                .into_client_request()
                .map_err(|e| format!("build request: {e}"))?;
            request.headers_mut().insert(
                tokio_tungstenite::tungstenite::http::header::HeaderName::from_static(
                    gemini::config::GOOGLE_API_KEY_HEADER,
                ),
                api_key
                    .parse()
                    .map_err(|e| format!("invalid x-goog-api-key header: {e}"))?,
            );
            let (ws, _) = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                tokio_tungstenite::connect_async(request),
            )
            .await
            .map_err(|_| "Connection timeout".to_string())?
            .map_err(|e| format!("Connection failed: {e}"))?;

            let (mut write, mut read) = ws.split();
            write
                .send(Message::Text(gemini::protocol::build_setup_message(
                    "en",
                    setup_options,
                )))
                .await
                .map_err(|e| format!("Failed to send setup: {e}"))?;

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
            while tokio::time::Instant::now() < deadline {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(ws_msg)) => {
                                let Some(text) = crate::ai::decode_ws_message(ws_msg) else {
                                    continue;
                                };
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(err) = gemini::protocol::parse_api_error(&value) {
                                        return Err(format!("Gemini API error: {err}"));
                                    }
                                    if value.get("setupComplete").is_some() {
                                        return Ok(());
                                    }
                                }
                            }
                            Some(Err(e)) => return Err(format!("WebSocket read error: {e}")),
                            None => return Err("WebSocket closed before setup".into()),
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
                }
            }
            Err("Gemini setup timeout — check API key has Live API access".into())
        }
        AiProvider::OpenAi => {
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;

            let url = crate::ai::translations_ws_url(crate::ai::DEFAULT_OPENAI_LIVE_MODEL);
            let mut req = url
                .into_client_request()
                .map_err(|e| format!("build request: {e}"))?;
            req.headers_mut().insert(
                "Authorization",
                format!("Bearer {api_key}")
                    .parse()
                    .map_err(|e| format!("invalid Authorization header: {e}"))?,
            );

            let (ws, _) = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                tokio_tungstenite::connect_async(req),
            )
            .await
            .map_err(|_| "Connection timeout".to_string())?
            .map_err(|e| format!("Connection failed: {e}"))?;

            let (mut write, mut read) = ws.split();
            write
                .send(Message::Text(openai::protocol::build_session_update(
                    "en",
                    "near_field",
                )))
                .await
                .map_err(|e| format!("Failed to send session.update: {e}"))?;

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
            while tokio::time::Instant::now() < deadline {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(ws_msg)) => {
                                let Some(text) = crate::ai::decode_ws_message(ws_msg) else {
                                    continue;
                                };
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(err) = openai::protocol::parse_api_error(&value) {
                                        return Err(format!("OpenAI API error: {err}"));
                                    }
                                    if matches!(
                                        value.get("type").and_then(|t| t.as_str()),
                                        Some("session.created") | Some("session.updated")
                                    ) {
                                        return Ok(());
                                    }
                                }
                            }
                            Some(Err(e)) => return Err(format!("WebSocket read error: {e}")),
                            None => return Err("WebSocket closed before session ready".into()),
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
                }
            }
            Err("OpenAI setup timeout — check API key has Realtime Translation access".into())
        }
        AiProvider::Soniox => {
            let setup = soniox::protocol::SonioxSttSetup {
                api_key: api_key.to_string(),
                model: soniox::config::DEFAULT_SONIOX_LIVE_MODEL.to_string(),
                target_language: "en".into(),
                language_hints: vec!["en".into()],
                context: Default::default(),
                endpoint_latency_adjustment_level:
                    soniox::config::ENDPOINT_LATENCY_ADJUSTMENT_LEVEL,
                endpoint_sensitivity: soniox::config::ENDPOINT_SENSITIVITY,
                max_endpoint_delay_ms: soniox::config::MAX_ENDPOINT_DELAY_MS,
                translation_enabled: setup_options.translation_enabled,
            };
            let (ws, _) = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                tokio_tungstenite::connect_async(soniox::config::stt_ws_url()),
            )
            .await
            .map_err(|_| "Connection timeout".to_string())?
            .map_err(|e| format!("Connection failed: {e}"))?;

            let (mut write, mut read) = ws.split();
            write
                .send(Message::Text(soniox::protocol::build_stt_config_message(
                    &setup,
                )))
                .await
                .map_err(|e| format!("Failed to send Soniox config: {e}"))?;

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
            while tokio::time::Instant::now() < deadline {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(ws_msg)) => {
                                let Some(text) = crate::ai::decode_ws_message(ws_msg) else {
                                    continue;
                                };
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(err) = soniox::protocol::parse_api_error(&value) {
                                        return Err(format!("Soniox API error: {err}"));
                                    }
                                    if value.get("error_type").is_some() {
                                        let msg = value
                                            .get("error_message")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("unknown");
                                        return Err(format!("Soniox API error: {msg}"));
                                    }
                                    return Ok(());
                                }
                            }
                            Some(Err(e)) => return Err(format!("WebSocket read error: {e}")),
                            None => return Err("WebSocket closed before Soniox ready".into()),
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(400)) => {
                        return Ok(());
                    }
                }
            }
            Ok(())
        }
    }
}
