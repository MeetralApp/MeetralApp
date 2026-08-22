use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde_json::Value;

use super::config::{
    rest_generate_content_url, rest_stream_generate_content_url, GOOGLE_API_KEY_HEADER,
};
use crate::ai::llm::{
    kind_from_http_status, kind_from_reqwest, ChatFuture, ChatLlmProvider, ChatRequest,
    ChatResponse, LlmError, LlmErrorKind, LlmUsage, ToolCallResponse, ToolChoice, ToolDefinition,
};
use crate::capabilities::{LlmCapabilities, LLM_CAPS_GEMINI};
use crate::providers::shared::sse::SseParser;

pub const SUMMARY_LLM_TIMEOUT_SECS: u64 = crate::ai::summary::SUMMARY_LLM_TIMEOUT_SECS;
pub const SUMMARY_STREAM_IDLE_TIMEOUT_SECS: u64 =
    crate::ai::summary::SUMMARY_STREAM_IDLE_TIMEOUT_SECS;

/// `ChatLlmProvider` for Gemini REST (`:generateContent` / `:streamGenerateContent`).
/// Model-agnostic: `ChatRequest::model` is authoritative; the constructor model
/// is only reported via `model_id()`.
pub struct GeminiChatLlm {
    api_key: String,
    model: String,
}

impl GeminiChatLlm {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }
}

impl ChatLlmProvider for GeminiChatLlm {
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
        &LLM_CAPS_GEMINI
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
        format!("Gemini API error {status}: {body}"),
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
    let url = rest_generate_content_url(model);
    let body = gemini_body(system, prompt, json_response, tools, tool_choice);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(SUMMARY_LLM_TIMEOUT_SECS))
        .build()
        .context("build http client")?;

    let resp = client
        .post(&url)
        .header(GOOGLE_API_KEY_HEADER, api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| reqwest_error("gemini generate request", err))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(api_error(status, &text));
    }

    let value: Value = resp.json().await.context("parse gemini response")?;
    parse_gemini_response(&value)
}

fn parse_gemini_response(value: &Value) -> Result<ChatResponse> {
    let usage = parse_usage_metadata(value.get("usageMetadata"));
    let (text, tool_calls) = parse_gemini_parts(value.pointer("/candidates/0/content/parts"));

    if tool_calls.is_empty() && text.is_empty() {
        anyhow::bail!("missing text in gemini response");
    }

    Ok(ChatResponse {
        text,
        usage,
        tool_call: None,
        tool_calls: Vec::new(),
    }
    .with_tool_calls(tool_calls))
}

fn parse_usage_metadata(meta: Option<&Value>) -> LlmUsage {
    let Some(meta) = meta else {
        return LlmUsage::default();
    };
    LlmUsage {
        prompt_tokens: meta
            .get("promptTokenCount")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        completion_tokens: meta
            .get("candidatesTokenCount")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        total_tokens: meta
            .get("totalTokenCount")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
    }
}

fn parse_gemini_parts(parts: Option<&Value>) -> (String, Vec<ToolCallResponse>) {
    let Some(parts) = parts.and_then(|p| p.as_array()) else {
        return (String::new(), Vec::new());
    };
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    for part in parts {
        if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
            text.push_str(t);
        }
        if let Some(fc) = part.get("functionCall") {
            if let Some(name) = fc.get("name").and_then(|n| n.as_str()) {
                let arguments = fc
                    .get("args")
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));
                tool_calls.push(ToolCallResponse {
                    name: name.to_string(),
                    arguments,
                });
            }
        }
    }
    (text, tool_calls)
}

/// Gemini `:generateContent` request body. Optional `systemInstruction` carries
/// the higher-privilege rule block; untrusted transcript lives in `contents`.
fn gemini_body(
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
    tools: Option<&[ToolDefinition]>,
    tool_choice: ToolChoice,
) -> Value {
    // Gemini rejects function calling combined with responseMimeType
    // application/json (`INVALID_ARGUMENT`). Prefer tools when both are set.
    let has_tools = tools.map(|t| !t.is_empty()).unwrap_or(false);
    let json_response = json_response && !has_tools;
    let generation_config = if json_response {
        serde_json::json!({
            "temperature": 0.3,
            "responseMimeType": "application/json",
            "thinkingConfig": { "thinkingBudget": 0 }
        })
    } else {
        serde_json::json!({
            "temperature": 0.3,
            "thinkingConfig": { "thinkingBudget": 0 }
        })
    };
    let mut body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": generation_config,
    });
    if let Some(system) = system {
        body["systemInstruction"] = serde_json::json!({ "parts": [{ "text": system }] });
    }
    if has_tools {
        if let Some(tools) = tools {
            let declarations: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!([{ "functionDeclarations": declarations }]);
            body["toolConfig"] = serde_json::json!({
                "functionCallingConfig": {
                    "mode": gemini_tool_choice_mode(tool_choice)
                }
            });
        }
    }
    body
}

fn gemini_tool_choice_mode(choice: ToolChoice) -> &'static str {
    match choice {
        ToolChoice::Auto => "AUTO",
        ToolChoice::Required => "ANY",
        ToolChoice::None => "NONE",
    }
}

/// Streaming variant of [`generate`] via `:streamGenerateContent?alt=sse`.
/// `on_delta` fires per text part as it arrives; returns the full concatenated text.
pub async fn generate_stream(
    api_key: &str,
    model: &str,
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
    mut on_delta: impl FnMut(&str) + Send,
) -> Result<String> {
    let url = rest_stream_generate_content_url(model);
    let body = gemini_body(system, prompt, json_response, None, ToolChoice::None);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(SUMMARY_LLM_TIMEOUT_SECS))
        .build()
        .context("build http client")?;

    let resp = client
        .post(&url)
        .header(GOOGLE_API_KEY_HEADER, api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| reqwest_error("gemini stream request", err))?;

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
            Ok(Some(Err(err))) => return Err(err).context("gemini stream read"),
            Ok(None) => break,
            Err(_) => {
                return Err(LlmError::new(
                    LlmErrorKind::Timeout,
                    format!(
                        "Gemini stream idle timeout (no chunk for {SUMMARY_STREAM_IDLE_TIMEOUT_SECS}s)"
                    ),
                )
                .into());
            }
        };
        for data in parser.push(&chunk) {
            if let Some(delta) = gemini_stream_delta(&data) {
                full.push_str(&delta);
                on_delta(&delta);
            }
        }
    }
    for data in parser.finish() {
        if let Some(delta) = gemini_stream_delta(&data) {
            full.push_str(&delta);
            on_delta(&delta);
        }
    }

    if full.is_empty() {
        anyhow::bail!("empty gemini stream response");
    }
    Ok(full)
}

/// Extract concatenated text parts from one SSE `data:` payload (pure, testable).
pub fn gemini_stream_delta(data: &str) -> Option<String> {
    let value: Value = serde_json::from_str(data).ok()?;
    let parts = value.pointer("/candidates/0/content/parts")?.as_array()?;
    let mut text = String::new();
    for part in parts {
        if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
            text.push_str(t);
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_extracts_single_part() {
        let data = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"}}]}"#;
        assert_eq!(gemini_stream_delta(data), Some("Hello".to_string()));
    }

    #[test]
    fn delta_concatenates_multiple_parts() {
        let data = r#"{"candidates":[{"content":{"parts":[{"text":"xin "},{"text":"chào"}]}}]}"#;
        assert_eq!(gemini_stream_delta(data), Some("xin chào".to_string()));
    }

    #[test]
    fn delta_ignores_metadata_only_events() {
        assert_eq!(
            gemini_stream_delta(r#"{"usageMetadata":{"totalTokenCount":12}}"#),
            None
        );
        assert_eq!(gemini_stream_delta("not json"), None);
        assert_eq!(
            gemini_stream_delta(r#"{"candidates":[{"finishReason":"STOP"}]}"#),
            None
        );
    }

    #[test]
    fn api_error_maps_status_to_kind() {
        let err = api_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "quota");
        let llm = err.downcast_ref::<LlmError>().expect("classified");
        assert_eq!(llm.kind, LlmErrorKind::RateLimited);
        // Message parity with the pre-taxonomy `bail!` — `StatusCode` Display
        // includes the canonical reason phrase.
        assert_eq!(
            format!("{err:#}"),
            "Gemini API error 429 Too Many Requests: quota"
        );

        let err = api_error(reqwest::StatusCode::SERVICE_UNAVAILABLE, "overloaded");
        assert_eq!(
            err.downcast_ref::<LlmError>().expect("classified").kind,
            LlmErrorKind::Server
        );
        let err = api_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "boom");
        assert_eq!(
            err.downcast_ref::<LlmError>().expect("classified").kind,
            LlmErrorKind::Server
        );
        let err = api_error(reqwest::StatusCode::FORBIDDEN, "key rejected");
        assert_eq!(
            err.downcast_ref::<LlmError>().expect("classified").kind,
            LlmErrorKind::Auth
        );
    }

    #[test]
    fn chat_llm_reports_caps_and_model() {
        let client = GeminiChatLlm::new("k", "gemini-2.5-flash");
        assert_eq!(client.model_id(), "gemini-2.5-flash");
        assert!(client.capabilities().streaming);
        assert!(client.capabilities().json_mode);
        assert!(client.supports_tools());
    }

    #[test]
    fn parse_response_text_and_usage() {
        let value = serde_json::json!({
            "candidates": [{"content": {"parts": [{"text": "hello"}]}}],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 3,
                "totalTokenCount": 13
            }
        });
        let resp = parse_gemini_response(&value).unwrap();
        assert_eq!(resp.text, "hello");
        assert_eq!(resp.usage.prompt_tokens, Some(10));
        assert_eq!(resp.usage.completion_tokens, Some(3));
        assert_eq!(resp.usage.total_tokens, Some(13));
        assert!(resp.tool_call.is_none());
    }

    #[test]
    fn parse_response_function_call() {
        let value = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "retrieve",
                            "args": {"query": "budget"}
                        }
                    }]
                }
            }]
        });
        let resp = parse_gemini_response(&value).unwrap();
        let tc = resp.tool_call.expect("tool call");
        assert_eq!(tc.name, "retrieve");
        assert_eq!(tc.arguments["query"], "budget");
        assert_eq!(resp.tool_calls.len(), 1);
    }

    #[test]
    fn parse_response_parallel_function_calls() {
        let value = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "functionCall": {
                                "name": "retrieve",
                                "args": {"query": "budget"}
                            }
                        },
                        {
                            "functionCall": {
                                "name": "extract",
                                "args": {"kind": "decisions"}
                            }
                        }
                    ]
                }
            }]
        });
        let resp = parse_gemini_response(&value).unwrap();
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
        let body = gemini_body(None, "hi", false, Some(&tools), ToolChoice::Auto);
        assert_eq!(
            body["tools"][0]["functionDeclarations"][0]["name"],
            "retrieve"
        );
        assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "AUTO");
    }

    #[test]
    fn body_drops_json_mime_when_tools_present() {
        let tools = [ToolDefinition {
            name: "retrieve".into(),
            description: "search".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let body = gemini_body(None, "hi", true, Some(&tools), ToolChoice::Auto);
        assert!(body["generationConfig"].get("responseMimeType").is_none());
        assert_eq!(
            body["tools"][0]["functionDeclarations"][0]["name"],
            "retrieve"
        );
    }

    #[test]
    fn body_keeps_json_mime_without_tools() {
        let body = gemini_body(None, "hi", true, None, ToolChoice::None);
        assert_eq!(
            body["generationConfig"]["responseMimeType"],
            "application/json"
        );
    }
}
