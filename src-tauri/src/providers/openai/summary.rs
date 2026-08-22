use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde_json::Value;

use super::config::chat_completions_url;
use crate::ai::llm::{
    kind_from_http_status, kind_from_reqwest, ChatFuture, ChatLlmProvider, ChatRequest,
    ChatResponse, LlmError, LlmErrorKind, LlmUsage, ToolCallResponse, ToolChoice, ToolDefinition,
};
use crate::ai::summary::{SUMMARY_LLM_TIMEOUT_SECS, SUMMARY_STREAM_IDLE_TIMEOUT_SECS};
use crate::capabilities::{LlmCapabilities, LLM_CAPS_OPENAI};
use crate::providers::shared::sse::SseParser;

/// `ChatLlmProvider` for OpenAI `chat/completions`. Model-agnostic:
/// `ChatRequest::model` is authoritative; the constructor model is only
/// reported via `model_id()`.
pub struct OpenAiChatLlm {
    api_key: String,
    model: String,
}

impl OpenAiChatLlm {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }
}

impl ChatLlmProvider for OpenAiChatLlm {
    fn generate<'a>(&'a self, req: &'a ChatRequest) -> ChatFuture<'a> {
        Box::pin(async move {
            generate(
                &self.api_key,
                req.model,
                req.system,
                req.prompt,
                req.json_response,
                req.tools,
                req.tool_choice,
            )
            .await
        })
    }

    fn generate_stream<'a>(
        &'a self,
        req: &'a ChatRequest,
        on_delta: Box<dyn FnMut(&str) + Send + 'a>,
    ) -> ChatFuture<'a> {
        Box::pin(async move {
            generate_stream(
                &self.api_key,
                req.model,
                req.system,
                req.prompt,
                req.json_response,
                on_delta,
            )
            .await
            .map(ChatResponse::from_text)
        })
    }

    fn capabilities(&self) -> &LlmCapabilities {
        &LLM_CAPS_OPENAI
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

/// Map a reqwest send failure to a classified `LlmError`, preserving the
/// legacy `"{context}: {source}"` message shape.
fn reqwest_error(context: &str, err: reqwest::Error) -> anyhow::Error {
    LlmError::new(kind_from_reqwest(&err), format!("{context}: {err}")).into()
}

/// Map a non-2xx response to a classified `LlmError` (status + vendor body).
fn api_error(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    LlmError::new(
        kind_from_http_status(status.as_u16()),
        format!("OpenAI API error {status}: {body}"),
    )
    .into()
}

pub async fn generate(
    api_key: &str,
    model: &str,
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
    tools: Option<&[ToolDefinition]>,
    tool_choice: ToolChoice,
) -> Result<ChatResponse> {
    let body = build_chat_body(
        model,
        system,
        prompt,
        json_response,
        false,
        tools,
        tool_choice,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(SUMMARY_LLM_TIMEOUT_SECS))
        .build()
        .context("build http client")?;

    let resp = client
        .post(chat_completions_url())
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| reqwest_error("openai chat request", err))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(api_error(status, &text));
    }

    let value: Value = resp.json().await.context("parse openai response")?;
    parse_openai_response(&value)
}

pub(crate) fn parse_openai_response(value: &Value) -> Result<ChatResponse> {
    let usage = parse_openai_usage(value.get("usage"));
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tool_calls = parse_openai_tool_calls(value.pointer("/choices/0/message/tool_calls"));

    if tool_calls.is_empty() && text.is_empty() {
        anyhow::bail!("missing text in openai response");
    }

    Ok(ChatResponse {
        text,
        usage,
        tool_call: None,
        tool_calls: Vec::new(),
    }
    .with_tool_calls(tool_calls))
}

pub(crate) fn parse_openai_usage(usage: Option<&Value>) -> LlmUsage {
    let Some(usage) = usage else {
        return LlmUsage::default();
    };
    LlmUsage {
        prompt_tokens: usage
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        total_tokens: usage
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
    }
}

fn parse_openai_tool_calls(tool_calls: Option<&Value>) -> Vec<ToolCallResponse> {
    let Some(arr) = tool_calls.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let function = item.get("function")?;
            let name = function.get("name")?.as_str()?.to_string();
            let arguments = match function.get("arguments") {
                Some(Value::String(s)) => {
                    serde_json::from_str(s).unwrap_or(Value::Object(Default::default()))
                }
                Some(other) => other.clone(),
                None => Value::Object(Default::default()),
            };
            Some(ToolCallResponse { name, arguments })
        })
        .collect()
}

/// OpenAI `chat/completions` request body. Emits an optional leading `system`
/// message before the `user` content (privilege separation for role rules).
fn build_chat_body(
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
        "messages": messages,
    });
    if stream {
        body["stream"] = serde_json::json!(true);
    }
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
            body["tool_choice"] = Value::String(openai_tool_choice(tool_choice).into());
        }
    }
    body
}

fn openai_tool_choice(choice: ToolChoice) -> &'static str {
    match choice {
        ToolChoice::Auto => "auto",
        ToolChoice::Required => "required",
        ToolChoice::None => "none",
    }
}

/// Streaming variant of [`generate`] via `chat/completions` with `stream: true`.
/// `on_delta` fires per content delta; returns the full concatenated text.
pub async fn generate_stream(
    api_key: &str,
    model: &str,
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
    mut on_delta: impl FnMut(&str) + Send,
) -> Result<String> {
    let body = build_chat_body(
        model,
        system,
        prompt,
        json_response,
        true,
        None,
        ToolChoice::None,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(SUMMARY_LLM_TIMEOUT_SECS))
        .build()
        .context("build http client")?;

    let resp = client
        .post(chat_completions_url())
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| reqwest_error("openai stream request", err))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(api_error(status, &text));
    }

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
            Ok(Some(Err(err))) => return Err(err).context("openai stream read"),
            Ok(None) => break,
            Err(_) => {
                return Err(LlmError::new(
                    LlmErrorKind::Timeout,
                    format!(
                        "OpenAI stream idle timeout (no chunk for {SUMMARY_STREAM_IDLE_TIMEOUT_SECS}s)"
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
        anyhow::bail!("empty openai stream response");
    }
    Ok(full)
}

/// Extract the content delta from one SSE `data:` payload (pure, testable).
/// Skips the `[DONE]` sentinel, role-only deltas, and usage chunks.
pub fn openai_stream_delta(data: &str) -> Option<String> {
    if data.trim() == "[DONE]" {
        return None;
    }
    let value: Value = serde_json::from_str(data).ok()?;
    let content = value.pointer("/choices/0/delta/content")?.as_str()?;
    if content.is_empty() {
        None
    } else {
        Some(content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_extracts_content() {
        let data = r#"{"choices":[{"delta":{"content":"Hello"},"index":0}]}"#;
        assert_eq!(openai_stream_delta(data), Some("Hello".to_string()));
    }

    #[test]
    fn delta_skips_done_role_only_and_usage() {
        assert_eq!(openai_stream_delta("[DONE]"), None);
        assert_eq!(
            openai_stream_delta(r#"{"choices":[{"delta":{"role":"assistant"},"index":0}]}"#),
            None
        );
        assert_eq!(openai_stream_delta(r#"{"choices":[],"usage":{}}"#), None);
        assert_eq!(openai_stream_delta("not json"), None);
    }

    #[test]
    fn delta_skips_empty_content() {
        let data = r#"{"choices":[{"delta":{"content":""},"index":0}]}"#;
        assert_eq!(openai_stream_delta(data), None);
    }

    #[test]
    fn api_error_maps_status_to_kind() {
        let err = api_error(reqwest::StatusCode::UNAUTHORIZED, "invalid key");
        let llm = err.downcast_ref::<LlmError>().expect("classified");
        assert_eq!(llm.kind, LlmErrorKind::Auth);
        // Message parity with the pre-taxonomy `bail!` — `StatusCode` Display
        // includes the canonical reason phrase.
        assert_eq!(
            format!("{err:#}"),
            "OpenAI API error 401 Unauthorized: invalid key"
        );

        let err = api_error(reqwest::StatusCode::NOT_FOUND, "unknown model");
        assert_eq!(
            err.downcast_ref::<LlmError>().expect("classified").kind,
            LlmErrorKind::InvalidRequest
        );
        let err = api_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "quota");
        assert_eq!(
            err.downcast_ref::<LlmError>().expect("classified").kind,
            LlmErrorKind::RateLimited
        );
        let err = api_error(reqwest::StatusCode::BAD_GATEWAY, "bad gateway");
        assert_eq!(
            err.downcast_ref::<LlmError>().expect("classified").kind,
            LlmErrorKind::Server
        );
    }

    #[test]
    fn chat_llm_reports_caps_and_model() {
        let client = OpenAiChatLlm::new("k", "gpt-4o");
        assert_eq!(client.model_id(), "gpt-4o");
        assert!(client.capabilities().streaming);
        assert!(client.capabilities().json_mode);
        assert!(client.supports_tools());
    }

    #[test]
    fn parse_response_text_and_usage() {
        let value = serde_json::json!({
            "choices": [{"message": {"content": "hello"}}],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 4,
                "total_tokens": 15
            }
        });
        let resp = parse_openai_response(&value).unwrap();
        assert_eq!(resp.text, "hello");
        assert_eq!(resp.usage.prompt_tokens, Some(11));
        assert_eq!(resp.usage.completion_tokens, Some(4));
        assert_eq!(resp.usage.total_tokens, Some(15));
    }

    #[test]
    fn parse_response_tool_call() {
        let value = serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "type": "function",
                        "function": {
                            "name": "retrieve",
                            "arguments": "{\"query\":\"budget\"}"
                        }
                    }]
                }
            }]
        });
        let resp = parse_openai_response(&value).unwrap();
        let tc = resp.tool_call.expect("tool call");
        assert_eq!(tc.name, "retrieve");
        assert_eq!(tc.arguments["query"], "budget");
        assert_eq!(resp.tool_calls.len(), 1);
    }

    #[test]
    fn parse_response_parallel_tool_calls() {
        let value = serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [
                        {
                            "type": "function",
                            "function": {
                                "name": "retrieve",
                                "arguments": "{\"query\":\"budget\"}"
                            }
                        },
                        {
                            "type": "function",
                            "function": {
                                "name": "extract",
                                "arguments": "{\"kind\":\"decisions\"}"
                            }
                        }
                    ]
                }
            }]
        });
        let resp = parse_openai_response(&value).unwrap();
        assert_eq!(resp.native_tool_calls().len(), 2);
        assert_eq!(resp.tool_calls[1].name, "extract");
    }

    #[test]
    fn body_includes_tools_when_present() {
        let tools = [ToolDefinition {
            name: "retrieve".into(),
            description: "search".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let body = build_chat_body(
            "gpt-4o",
            None,
            "hi",
            false,
            false,
            Some(&tools),
            ToolChoice::Required,
        );
        assert_eq!(body["tools"][0]["function"]["name"], "retrieve");
        assert_eq!(body["tool_choice"], "required");
    }
}
