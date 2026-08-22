//! Chat completions for OpenAI-compatible servers. Request/response shape
//! mirrors `providers::openai::summary` (same `chat/completions` wire, same
//! SSE delta shape) with two differences: the base URL is per-profile
//! (Configurable) and the API key is optional (local servers often need
//! none — `LlmAuthMode::Optional`).

use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde_json::Value;

use crate::ai::llm::{
    kind_from_http_status, kind_from_reqwest, ChatFuture, ChatLlmProvider, ChatRequest,
    ChatResponse, LlmError, LlmErrorKind, ToolChoice, ToolDefinition,
};
use crate::ai::summary::{SUMMARY_LLM_TIMEOUT_SECS, SUMMARY_STREAM_IDLE_TIMEOUT_SECS};
use crate::capabilities::{LlmCapabilities, LLM_CAPS_COMPATIBLE};
use crate::providers::openai::summary::{openai_stream_delta, parse_openai_response};
use crate::providers::shared::sse::SseParser;

/// `ChatLlmProvider` for custom OpenAI-compatible profiles. Model-agnostic
/// like the built-ins: `ChatRequest::model` is authoritative (the resolver
/// always passes `profile.chat_model`); the constructor model backs
/// `model_id()` for observability.
pub struct CompatibleChatLlm {
    base_url: String,
    api_key: Option<String>,
    model: String,
    caps: LlmCapabilities,
}

impl CompatibleChatLlm {
    pub fn new(base_url: &str, api_key: Option<&str>, model: &str, json_mode: bool) -> Self {
        let mut caps = LLM_CAPS_COMPATIBLE;
        caps.json_mode = json_mode;
        Self {
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            api_key: api_key
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .map(str::to_string),
            model: model.to_string(),
            caps,
        }
    }
}

fn chat_completions_url(base_url: &str) -> String {
    super::endpoint_url(base_url, "chat/completions")
}

impl ChatLlmProvider for CompatibleChatLlm {
    fn generate<'a>(&'a self, req: &'a ChatRequest) -> ChatFuture<'a> {
        Box::pin(async move {
            let use_json = self.caps.json_mode && req.json_response;
            let body = chat_body(
                req.model,
                req.system,
                req.prompt,
                use_json,
                false,
                req.tools,
                req.tool_choice,
            );
            send(&self.base_url, self.api_key.as_deref(), &body, "chat").await
        })
    }

    fn generate_stream<'a>(
        &'a self,
        req: &'a ChatRequest,
        mut on_delta: Box<dyn FnMut(&str) + Send + 'a>,
    ) -> ChatFuture<'a> {
        Box::pin(async move {
            let use_json = self.caps.json_mode && req.json_response;
            let body = chat_body(
                req.model,
                req.system,
                req.prompt,
                use_json,
                true,
                None,
                ToolChoice::None,
            );
            let resp =
                send_streaming(&self.base_url, self.api_key.as_deref(), &body, "stream").await?;

            let mut parser = SseParser::new();
            let mut full = String::new();
            let mut byte_stream = resp.bytes_stream();
            loop {
                let chunk = match tokio::time::timeout(
                    std::time::Duration::from_secs(SUMMARY_STREAM_IDLE_TIMEOUT_SECS),
                    byte_stream.next(),
                )
                .await
                {
                    Ok(Some(Ok(bytes))) => bytes,
                    Ok(Some(Err(err))) => return Err(err).context("compatible stream read"),
                    Ok(None) => break,
                    Err(_) => {
                        return Err(LlmError::new(
                            LlmErrorKind::Timeout,
                            format!(
                                "Compatible stream idle timeout (no chunk for {SUMMARY_STREAM_IDLE_TIMEOUT_SECS}s)"
                            ),
                        )
                        .into());
                    }
                };
                for data in parser.push(&chunk) {
                    if let Some(delta) = openai_stream_delta(&data) {
                        full.push_str(&delta);
                        on_delta(&delta);
                    }
                }
            }
            for data in parser.finish() {
                if let Some(delta) = openai_stream_delta(&data) {
                    full.push_str(&delta);
                    on_delta(&delta);
                }
            }

            if full.is_empty() {
                anyhow::bail!("empty compatible stream response");
            }
            Ok(ChatResponse::from_text(full))
        })
    }

    fn capabilities(&self) -> &LlmCapabilities {
        &self.caps
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

/// OpenAI `chat/completions` body — identical shape to the built-in OpenAI
/// slice (temperature 0.3, optional `response_format`). `stream` is always
/// explicit: some proxies treat a missing field as "server default" and
/// stream anyway (observed with OmniRoute).
fn chat_body(
    model: &str,
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
    stream: bool,
    tools: Option<&[ToolDefinition]>,
    tool_choice: ToolChoice,
) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = system {
        messages.push(serde_json::json!({ "role": "system", "content": system }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": prompt }));
    let mut body = serde_json::json!({
        "model": model,
        "temperature": 0.3,
        "stream": stream,
        "messages": messages,
    });
    if json_response {
        body["response_format"] = serde_json::json!({ "type": "json_object" });
    }
    if let Some(tools) = tools {
        if !tools.is_empty() {
            let tools_json: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect();
            body["tools"] = Value::Array(tools_json);
            body["tool_choice"] = Value::String(
                match tool_choice {
                    ToolChoice::Auto => "auto",
                    ToolChoice::Required => "required",
                    ToolChoice::None => "none",
                }
                .into(),
            );
        }
    }
    body
}

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(SUMMARY_LLM_TIMEOUT_SECS))
        .build()
        .context("build http client")
}

fn apply_auth(request: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    match api_key {
        Some(key) => request.bearer_auth(key),
        None => request,
    }
}

/// Map a reqwest send failure to a classified `LlmError` (connection refused
/// / DNS → `Unavailable`, timeout → `Timeout`).
fn reqwest_error(context: &str, err: reqwest::Error) -> anyhow::Error {
    LlmError::new(kind_from_reqwest(&err), format!("{context}: {err}")).into()
}

/// Map a non-2xx response to a classified `LlmError` (401/403 →
/// Auth, 404 → InvalidRequest, 429 → RateLimited, 5xx → Server).
fn api_error(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    LlmError::new(
        kind_from_http_status(status.as_u16()),
        format!("Compatible API error {status}: {body}"),
    )
    .into()
}

async fn send(
    base_url: &str,
    api_key: Option<&str>,
    body: &Value,
    context: &str,
) -> Result<ChatResponse> {
    let resp = apply_auth(http_client()?.post(chat_completions_url(base_url)), api_key)
        .json(body)
        .send()
        .await
        .map_err(|err| reqwest_error(&format!("compatible {context} request"), err))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(api_error(status, &text));
    }
    // Read as text first: a 200 with a non-JSON body (HTML error page, empty
    // body, or SSE despite stream:false — OmniRoute) must be handled explicitly.
    let text = resp.text().await.context("read compatible response body")?;
    parse_chat_response_body(&text)
}

/// Accept both OpenAI response shapes:
/// - classic JSON `{choices:[{message:{content}}]}` when `stream:false` is honored
/// - SSE (`data: {...delta...}`) when a proxy forces streaming anyway
fn parse_chat_response_body(text: &str) -> Result<ChatResponse> {
    let trimmed = text.trim_start();
    if trimmed.starts_with("data:") || trimmed.starts_with(':') {
        return assemble_sse_content(trimmed).map(ChatResponse::from_text);
    }
    let value: Value = serde_json::from_str(trimmed).with_context(|| {
        format!(
            "parse compatible response — body is not JSON: {}",
            super::body_excerpt(text)
        )
    })?;
    parse_openai_response(&value)
}

/// Drain a complete SSE body (already buffered) into the concatenated
/// `delta.content` text. Same wire shape as `generate_stream`.
fn assemble_sse_content(body: &str) -> Result<String> {
    let mut parser = SseParser::new();
    let mut full = String::new();
    for data in parser.push(body.as_bytes()) {
        if let Some(delta) = openai_stream_delta(&data) {
            full.push_str(&delta);
        }
    }
    for data in parser.finish() {
        if let Some(delta) = openai_stream_delta(&data) {
            full.push_str(&delta);
        }
    }
    if full.is_empty() {
        anyhow::bail!(
            "compatible server streamed an empty response (SSE with no content deltas) — body: {}",
            super::body_excerpt(body)
        );
    }
    Ok(full)
}

async fn send_streaming(
    base_url: &str,
    api_key: Option<&str>,
    body: &Value,
    context: &str,
) -> Result<reqwest::Response> {
    let resp = apply_auth(http_client()?.post(chat_completions_url(base_url)), api_key)
        .json(body)
        .send()
        .await
        .map_err(|err| reqwest_error(&format!("compatible {context} request"), err))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(api_error(status, &text));
    }
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_joins_without_double_slash() {
        assert_eq!(
            chat_completions_url("http://localhost:11434/"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("http://localhost:11434"),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn url_does_not_duplicate_v1_prefix() {
        // SDK-style base (OmniRoute, LM Studio docs) already ends in /v1.
        assert_eq!(
            chat_completions_url("http://localhost:20128/v1"),
            "http://localhost:20128/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("http://localhost:20128/v1/"),
            "http://localhost:20128/v1/chat/completions"
        );
    }

    #[test]
    fn empty_key_is_treated_as_absent() {
        let client = CompatibleChatLlm::new("http://localhost:11434", Some("  "), "m", true);
        assert_eq!(client.api_key, None);
    }

    #[test]
    fn body_matches_openai_shape() {
        let body = chat_body("qwen3:8b", None, "hi", true, true, None, ToolChoice::None);
        assert_eq!(body["model"], "qwen3:8b");
        assert_eq!(body["stream"], true);
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["messages"][0]["role"], "user");

        // stream is always explicit (false) so proxies can't default to SSE.
        let plain = chat_body("m", None, "hi", false, false, None, ToolChoice::None);
        assert_eq!(plain["stream"], false);
        assert!(plain.get("response_format").is_none());
    }

    #[test]
    fn body_omits_response_format_when_json_flag_false() {
        let body = chat_body(
            "m",
            None,
            "Reply with JSON only: {\"ok\":true}",
            false,
            false,
            None,
            ToolChoice::None,
        );
        assert!(body.get("response_format").is_none());
        assert_eq!(body["messages"][0]["role"], "user");
    }

    #[test]
    fn body_prepends_system_message_when_present() {
        let body = chat_body(
            "m",
            Some("You are an assistant."),
            "hi",
            false,
            false,
            None,
            ToolChoice::None,
        );
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "You are an assistant.");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn body_includes_tools_when_present() {
        let tools = [ToolDefinition {
            name: "retrieve".into(),
            description: "search".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let body = chat_body(
            "m",
            None,
            "hi",
            false,
            false,
            Some(&tools),
            ToolChoice::Auto,
        );
        assert_eq!(body["tools"][0]["function"]["name"], "retrieve");
        assert_eq!(body["tool_choice"], "auto");
    }

    #[test]
    fn parse_classic_json_message() {
        let body = r#"{"choices":[{"message":{"content":"{\"ok\":true}"}}],"usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}"#;
        let resp = parse_chat_response_body(body).unwrap();
        assert_eq!(resp.text, r#"{"ok":true}"#);
        assert_eq!(resp.usage.prompt_tokens, Some(1));
        assert_eq!(resp.usage.completion_tokens, Some(2));
        assert_eq!(resp.usage.total_tokens, Some(3));
    }

    #[test]
    fn parse_sse_when_server_forces_stream() {
        // OmniRoute / some proxies ignore stream:false and return SSE anyway.
        // reasoning_content deltas are ignored; only content contributes.
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"reasoning_content\":\"We\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"{\\\"ok\\\"\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\":true}\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let resp = parse_chat_response_body(body).unwrap();
        assert_eq!(resp.text, r#"{"ok":true}"#);
    }

    #[test]
    fn parse_sse_empty_content_is_clear_error() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"reasoning_content\":\"think\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let err = parse_chat_response_body(body).unwrap_err().to_string();
        assert!(err.contains("empty response"), "unexpected error: {err}");
    }

    #[test]
    fn api_error_maps_status_to_kind() {
        let cases: &[(reqwest::StatusCode, LlmErrorKind)] = &[
            (reqwest::StatusCode::UNAUTHORIZED, LlmErrorKind::Auth),
            (reqwest::StatusCode::FORBIDDEN, LlmErrorKind::Auth),
            (reqwest::StatusCode::NOT_FOUND, LlmErrorKind::InvalidRequest),
            (
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                LlmErrorKind::RateLimited,
            ),
            (
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                LlmErrorKind::Server,
            ),
        ];
        for (status, kind) in cases {
            let err = api_error(*status, "body");
            assert_eq!(
                err.downcast_ref::<LlmError>().expect("classified").kind,
                *kind,
                "status {status}"
            );
        }
    }

    #[test]
    fn chat_llm_reports_compatible_caps() {
        let client = CompatibleChatLlm::new("http://localhost:11434", None, "qwen3:8b", true);
        assert_eq!(client.model_id(), "qwen3:8b");
        let caps = client.capabilities();
        assert_eq!(caps.auth, crate::capabilities::LlmAuthMode::Optional);
        assert_eq!(caps.base_url, crate::capabilities::LlmBaseUrl::Configurable);
        assert_eq!(
            caps.model_source,
            crate::capabilities::ModelSource::Freeform
        );
        assert!(caps.json_mode);
        assert!(client.supports_tools());
    }

    #[test]
    fn chat_llm_can_disable_json_mode_cap() {
        let client = CompatibleChatLlm::new("http://localhost:11434", None, "m", false);
        assert!(!client.capabilities().json_mode);
    }
}
