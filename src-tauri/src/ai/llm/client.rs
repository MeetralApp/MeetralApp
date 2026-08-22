//! Object-safe chat-LLM trait + error taxonomy for summary generation.

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use serde_json::Value;

use crate::capabilities::LlmCapabilities;

pub type ChatFuture<'a> = Pin<Box<dyn Future<Output = Result<ChatResponse>> + Send + 'a>>;

/// Token usage reported by the vendor (all fields optional — some servers omit).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LlmUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

impl LlmUsage {
    pub fn estimated_cost_usd(&self, provider_label: &str, model: &str) -> f64 {
        let (input_per_1k, output_per_1k) = cost_rates(provider_label, model);
        let prompt = self.prompt_tokens.unwrap_or(0) as f64 * input_per_1k / 1000.0;
        let completion = self.completion_tokens.unwrap_or(0) as f64 * output_per_1k / 1000.0;
        prompt + completion
    }
}

fn cost_rates(provider_label: &str, model: &str) -> (f64, f64) {
    let p = provider_label.to_ascii_lowercase();
    let m = model.to_ascii_lowercase();
    if p.contains("gemini") {
        return (0.00015, 0.00060);
    }
    if p.contains("openai") || p.contains("gpt") {
        if m.contains("mini") {
            return (0.00015, 0.00060);
        }
        return (0.00250, 0.01000);
    }
    (0.0, 0.0)
}

/// Native function-calling tool definition (JSON Schema args).
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolChoice {
    #[default]
    Auto,
    Required,
    None,
}

/// Tool call returned by a native function-calling response.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallResponse {
    pub name: String,
    pub arguments: Value,
}

/// Full chat completion result (text and/or native tool calls + usage).
#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
    pub text: String,
    pub usage: LlmUsage,
    /// First native tool call — kept for call sites that only inspect one.
    /// Prefer [`Self::native_tool_calls`] when the turn may request several.
    pub tool_call: Option<ToolCallResponse>,
    /// All native tool calls from this turn (parallel function calling).
    pub tool_calls: Vec<ToolCallResponse>,
}

impl ChatResponse {
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            usage: LlmUsage::default(),
            tool_call: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn with_tool_calls(mut self, calls: Vec<ToolCallResponse>) -> Self {
        self.tool_call = calls.first().cloned();
        self.tool_calls = calls;
        self
    }

    /// Native tool calls for this turn. Prefers `tool_calls`; falls back to
    /// the singular `tool_call` field when the vec is empty.
    pub fn native_tool_calls(&self) -> &[ToolCallResponse] {
        if !self.tool_calls.is_empty() {
            &self.tool_calls
        } else if let Some(tc) = &self.tool_call {
            std::slice::from_ref(tc)
        } else {
            &[]
        }
    }
}

/// One chat completion request. `model` is authoritative per call.
pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub system: Option<&'a str>,
    pub prompt: &'a str,
    pub json_response: bool,
    pub tools: Option<&'a [ToolDefinition]>,
    pub tool_choice: ToolChoice,
}

impl<'a> ChatRequest<'a> {
    pub fn plain(
        model: &'a str,
        system: Option<&'a str>,
        prompt: &'a str,
        json_response: bool,
    ) -> Self {
        Self {
            model,
            system,
            prompt,
            json_response,
            tools: None,
            tool_choice: ToolChoice::None,
        }
    }
}

pub trait ChatLlmProvider: Send + Sync {
    fn generate<'a>(&'a self, req: &'a ChatRequest) -> ChatFuture<'a>;
    fn generate_stream<'a>(
        &'a self,
        req: &'a ChatRequest,
        on_delta: Box<dyn FnMut(&str) + Send + 'a>,
    ) -> ChatFuture<'a>;
    fn capabilities(&self) -> &LlmCapabilities;
    fn model_id(&self) -> &str;
    fn supports_tools(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmErrorKind {
    Timeout,
    RateLimited,
    Server,
    Auth,
    InvalidRequest,
    Unavailable,
}

impl LlmErrorKind {
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::RateLimited | Self::Server | Self::Unavailable
        )
    }
}

#[derive(Debug)]
pub struct LlmError {
    pub kind: LlmErrorKind,
    pub message: String,
}

impl LlmError {
    pub fn new(kind: LlmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LlmError {}

pub fn kind_from_http_status(status: u16) -> LlmErrorKind {
    match status {
        408 => LlmErrorKind::Timeout,
        401 | 403 => LlmErrorKind::Auth,
        429 => LlmErrorKind::RateLimited,
        500..=599 => LlmErrorKind::Server,
        _ => LlmErrorKind::InvalidRequest,
    }
}

pub fn kind_from_reqwest(err: &reqwest::Error) -> LlmErrorKind {
    if err.is_timeout() {
        LlmErrorKind::Timeout
    } else {
        LlmErrorKind::Unavailable
    }
}

pub fn is_retryable_llm_error(err: &anyhow::Error) -> bool {
    if let Some(llm_err) = err.downcast_ref::<LlmError>() {
        return llm_err.kind.is_retryable();
    }
    legacy_stringly_retryable(err)
}

fn legacy_stringly_retryable(err: &anyhow::Error) -> bool {
    let msg = format!("{err:#}").to_lowercase();
    msg.contains("timed out")
        || msg.contains("timeout")
        || msg.contains("operation timed out")
        || msg.contains("429")
        || msg.contains("rate limit")
        || msg.contains("503")
        || msg.contains("502")
}

pub struct FailingChatLlm {
    message: String,
    capabilities: LlmCapabilities,
}

impl FailingChatLlm {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            capabilities: unsupported_capabilities(),
        }
    }
}

fn unsupported_capabilities() -> LlmCapabilities {
    LlmCapabilities {
        streaming: false,
        json_mode: false,
        auth: crate::capabilities::LlmAuthMode::None,
        base_url: crate::capabilities::LlmBaseUrl::Fixed,
        model_source: crate::capabilities::ModelSource::Allowlist,
        context_window_tokens: None,
        reasoning: false,
        tool_calling: false,
    }
}

impl ChatLlmProvider for FailingChatLlm {
    fn generate<'a>(&'a self, _req: &'a ChatRequest) -> ChatFuture<'a> {
        Box::pin(async move { Err(anyhow::anyhow!("{}", self.message)) })
    }

    fn generate_stream<'a>(
        &'a self,
        _req: &'a ChatRequest,
        _on_delta: Box<dyn FnMut(&str) + Send + 'a>,
    ) -> ChatFuture<'a> {
        Box::pin(async move { Err(anyhow::anyhow!("{}", self.message)) })
    }

    fn capabilities(&self) -> &LlmCapabilities {
        &self.capabilities
    }

    fn model_id(&self) -> &str {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping_covers_vendor_cases() {
        assert_eq!(kind_from_http_status(408), LlmErrorKind::Timeout);
        assert_eq!(kind_from_http_status(401), LlmErrorKind::Auth);
        assert_eq!(kind_from_http_status(429), LlmErrorKind::RateLimited);
        assert_eq!(kind_from_http_status(500), LlmErrorKind::Server);
    }

    #[test]
    fn retryable_kinds() {
        assert!(LlmErrorKind::Timeout.is_retryable());
        assert!(!LlmErrorKind::Auth.is_retryable());
    }

    #[test]
    fn usage_cost_zero_for_compatible() {
        let usage = LlmUsage {
            prompt_tokens: Some(1000),
            completion_tokens: Some(500),
            total_tokens: Some(1500),
        };
        assert_eq!(usage.estimated_cost_usd("custom:local", "llama3"), 0.0);
        assert!(usage.estimated_cost_usd("Gemini", "gemini-2.5-flash") > 0.0);
    }
}
