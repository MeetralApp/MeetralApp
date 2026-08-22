//! Summary LLM client — retry/observability wrapper around `ChatLlmProvider`.
//! Behavior parity with the previous client: `generate` stays a
//! plain delegate (retry lives in `generate_with_retry`), streams retry once
//! only when the failure happened before the first delta, and `debug_log`
//! fields are unchanged. The only delta is the retry decision: classified
//! `LlmErrorKind` first, legacy string matcher as fallback for non-LLM errors.

use std::time::Instant;

use anyhow::Result;

use crate::ai::llm::{
    is_retryable_llm_error, ChatLlmProvider, ChatRequest, FailingChatLlm, LlmUsage,
};
use crate::ai::provider::AiProvider;
use crate::debug_log;

pub struct SummaryLlmClient {
    inner: Box<dyn ChatLlmProvider>,
    /// Built-in provider this client was created for; `None` for custom
    /// profiles (used for language gates + observability).
    provider: Option<AiProvider>,
    /// Profile id for custom clients (observability).
    custom_label: Option<String>,
}

impl SummaryLlmClient {
    /// Wrap the chat LLM for a built-in provider. Infallible: providers
    /// without a chat LLM (Soniox) get a stub that fails at call time with
    /// the load-bearing "no summary LLM" message, the same as the previous
    /// factory error surfaced at call time.
    pub fn for_provider(provider: AiProvider, api_key: &str) -> Self {
        let inner = crate::runtime::factories::summary::chat_llm_for(provider, api_key, "")
            .unwrap_or_else(|err| Box::new(FailingChatLlm::new(format!("{err:#}"))));
        Self {
            inner,
            provider: Some(provider),
            custom_label: None,
        }
    }

    /// Wrap an already-constructed provider (custom OpenAI-compatible
    /// profile). `label` is the profile id — logs show `custom:<id>`.
    pub fn custom(inner: Box<dyn ChatLlmProvider>, label: &str) -> Self {
        Self {
            inner,
            provider: None,
            custom_label: Some(label.to_string()),
        }
    }

    /// Language allowlist for built-in summary providers. Custom profiles
    /// skip the catalog gate (self-hosted models are assumed multilingual).
    pub fn supports_summary_language(&self, code: &str) -> bool {
        match self.provider {
            Some(provider) => crate::ai::is_supported_language_for_provider(provider, code),
            None => true,
        }
    }

    #[cfg(test)]
    fn builtin_provider(&self) -> Option<AiProvider> {
        self.provider
    }

    /// Human-readable origin for logs: matches the previous `?provider`
    /// debug format for built-ins, `custom:<id>` for profiles.
    pub fn origin_label(&self) -> String {
        match (self.provider, &self.custom_label) {
            (Some(provider), _) => format!("{provider:?}"),
            (None, Some(label)) => format!("custom:{label}"),
            (None, None) => "unknown".to_string(),
        }
    }

    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        json_response: bool,
        system: Option<&str>,
    ) -> Result<String> {
        Ok(self
            .generate_with_usage(model, prompt, json_response, system)
            .await?
            .0)
    }

    /// Like [`generate`], but also returns vendor token usage when available.
    pub async fn generate_with_usage(
        &self,
        model: &str,
        prompt: &str,
        json_response: bool,
        system: Option<&str>,
    ) -> Result<(String, LlmUsage)> {
        let response = self
            .generate_chat(model, prompt, json_response, system, None)
            .await?;
        Ok((response.text, response.usage))
    }

    /// Full chat result (text / native tool_call / usage). Pass `tools` for
    /// native function calling when [`Self::supports_tools`] is true.
    pub async fn generate_chat(
        &self,
        model: &str,
        prompt: &str,
        json_response: bool,
        system: Option<&str>,
        tools: Option<&[crate::ai::llm::ToolDefinition]>,
    ) -> Result<crate::ai::llm::ChatResponse> {
        let tool_choice = if tools.is_some() {
            crate::ai::llm::ToolChoice::Auto
        } else {
            crate::ai::llm::ToolChoice::None
        };
        self.inner
            .generate(&ChatRequest {
                model,
                system,
                prompt,
                json_response,
                tools,
                tool_choice,
            })
            .await
    }

    pub fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }

    /// Streaming generate: `on_delta` fires per chunk; returns full text.
    /// Retry policy: never retry after the first delta was emitted (the UI
    /// already received partial text); one retry allowed if the stream failed
    /// before producing any output.
    pub async fn generate_stream(
        &self,
        model: &str,
        prompt: &str,
        json_response: bool,
        system: Option<&str>,
        mut on_delta: impl FnMut(&str) + Send,
    ) -> Result<String> {
        let total_started = Instant::now();
        let origin = self.origin_label();
        debug_log!(
            provider = %origin,
            model,
            json = json_response,
            prompt_chars = prompt.len(),
            prompt,
            "llm stream start"
        );
        let request = ChatRequest::plain(model, system, prompt, json_response);
        for attempt in 1..=2 {
            let attempt_started = Instant::now();
            let mut emitted_any = false;
            let mut delta_count = 0u32;
            let mut ttft_ms: Option<u128> = None;
            let result = self
                .inner
                .generate_stream(
                    &request,
                    Box::new(|delta: &str| {
                        if !emitted_any {
                            ttft_ms = Some(attempt_started.elapsed().as_millis());
                        }
                        emitted_any = true;
                        delta_count += 1;
                        on_delta(delta);
                    }),
                )
                .await;
            match result {
                Ok(response) => {
                    let text = response.text;
                    let usage = &response.usage;
                    let estimated_cost_usd = usage.estimated_cost_usd(&origin, model);
                    debug_log!(
                        provider = %origin,
                        model,
                        attempt,
                        ttft_ms,
                        delta_count,
                        prompt_tokens = ?usage.prompt_tokens,
                        completion_tokens = ?usage.completion_tokens,
                        total_tokens = ?usage.total_tokens,
                        estimated_cost_usd,
                        duration_ms = attempt_started.elapsed().as_millis(),
                        total_ms = total_started.elapsed().as_millis(),
                        response_chars = text.len(),
                        response = text.as_str(),
                        "llm stream ok"
                    );
                    return Ok(text);
                }
                Err(err) => {
                    let retryable = is_retryable_llm_error(&err);
                    let will_retry = !emitted_any && attempt < 2 && retryable;
                    debug_log!(
                        provider = %origin,
                        model,
                        attempt,
                        emitted_any,
                        delta_count,
                        retryable,
                        will_retry,
                        duration_ms = attempt_started.elapsed().as_millis(),
                        error = %err,
                        "llm stream fail"
                    );
                    if emitted_any || attempt >= 2 || !retryable {
                        return Err(err);
                    }
                    tracing::warn!(
                        "summary stream {} failed before first delta, retrying: {err:#}",
                        self.origin_label()
                    );
                }
            }
        }
        unreachable!("retry loop returns on final attempt")
    }

    pub async fn generate_with_retry(
        &self,
        model: &str,
        prompt: &str,
        json_response: bool,
        system: Option<&str>,
        max_attempts: u32,
    ) -> Result<String> {
        let origin = self.origin_label();
        debug_log!(
            provider = %origin,
            model,
            json = json_response,
            max_attempts,
            prompt_chars = prompt.len(),
            prompt,
            "llm generate start"
        );
        let mut last_err = None;
        for attempt in 1..=max_attempts {
            let started = Instant::now();
            match self
                .generate_with_usage(model, prompt, json_response, system)
                .await
            {
                Ok((text, usage)) => {
                    let estimated_cost_usd = usage.estimated_cost_usd(&origin, model);
                    debug_log!(
                        provider = %origin,
                        model,
                        attempt,
                        max_attempts,
                        prompt_tokens = ?usage.prompt_tokens,
                        completion_tokens = ?usage.completion_tokens,
                        total_tokens = ?usage.total_tokens,
                        estimated_cost_usd,
                        duration_ms = started.elapsed().as_millis(),
                        response_chars = text.len(),
                        response = text.as_str(),
                        "llm generate ok"
                    );
                    return Ok(text);
                }
                Err(err) => {
                    let retryable = is_retryable_llm_error(&err);
                    debug_log!(
                        provider = %origin,
                        model,
                        attempt,
                        max_attempts,
                        retryable,
                        duration_ms = started.elapsed().as_millis(),
                        error = %err,
                        "llm generate fail"
                    );
                    tracing::warn!(
                        "summary {} attempt {attempt}/{max_attempts} failed: {err:#}",
                        self.origin_label()
                    );
                    last_err = Some(err);
                    if !retryable || attempt >= max_attempts {
                        break;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("summary request failed")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::llm::{ChatFuture, ChatResponse, LlmError, LlmErrorKind};
    use crate::capabilities::{LlmCapabilities, LLM_CAPS_OPENAI};
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
    use std::sync::Arc;

    /// Scripted fake — fails with a classified
    /// `LlmError` a fixed number of times, then succeeds.
    struct StubChatLlm {
        kind: LlmErrorKind,
        failures_before_success: u32,
        attempts: Arc<AtomicU32>,
    }

    impl StubChatLlm {
        fn failing(kind: LlmErrorKind, failures: u32) -> (Self, Arc<AtomicU32>) {
            let attempts = Arc::new(AtomicU32::new(0));
            (
                Self {
                    kind,
                    failures_before_success: failures,
                    attempts: Arc::clone(&attempts),
                },
                attempts,
            )
        }
    }

    impl ChatLlmProvider for StubChatLlm {
        fn generate<'a>(&'a self, _req: &'a ChatRequest) -> ChatFuture<'a> {
            let attempts = Arc::clone(&self.attempts);
            let failures = self.failures_before_success;
            let kind = self.kind;
            Box::pin(async move {
                let n = attempts.fetch_add(1, AtomicOrdering::SeqCst);
                if n < failures {
                    Err(LlmError::new(kind, "stub failure").into())
                } else {
                    Ok(ChatResponse::from_text("ok"))
                }
            })
        }

        fn generate_stream<'a>(
            &'a self,
            _req: &'a ChatRequest,
            mut on_delta: Box<dyn FnMut(&str) + Send + 'a>,
        ) -> ChatFuture<'a> {
            let attempts = Arc::clone(&self.attempts);
            let failures = self.failures_before_success;
            let kind = self.kind;
            Box::pin(async move {
                let n = attempts.fetch_add(1, AtomicOrdering::SeqCst);
                if n < failures {
                    Err(LlmError::new(kind, "stub stream failure").into())
                } else {
                    on_delta("chunk");
                    Ok(ChatResponse::from_text("chunk"))
                }
            })
        }

        fn capabilities(&self) -> &LlmCapabilities {
            &LLM_CAPS_OPENAI
        }

        fn model_id(&self) -> &str {
            "stub"
        }
    }

    #[tokio::test]
    async fn generate_with_retry_retries_retryable_kind_then_succeeds() {
        let (stub, attempts) = StubChatLlm::failing(LlmErrorKind::Timeout, 1);
        let client = SummaryLlmClient::custom(Box::new(stub), "test");
        let text = client
            .generate_with_retry("m", "p", false, None, 2)
            .await
            .expect("succeeds after one retry");
        assert_eq!(text, "ok");
        assert_eq!(attempts.load(AtomicOrdering::SeqCst), 2);
    }

    #[tokio::test]
    async fn generate_with_retry_does_not_retry_auth() {
        let (stub, attempts) = StubChatLlm::failing(LlmErrorKind::Auth, 10);
        let client = SummaryLlmClient::custom(Box::new(stub), "test");
        let err = client
            .generate_with_retry("m", "p", false, None, 3)
            .await
            .expect_err("auth is not retryable");
        assert!(format!("{err:#}").contains("stub failure"));
        assert_eq!(attempts.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stream_retries_once_when_failure_precedes_first_delta() {
        let (stub, attempts) = StubChatLlm::failing(LlmErrorKind::Server, 1);
        let client = SummaryLlmClient::custom(Box::new(stub), "test");
        let mut deltas = 0u32;
        let text = client
            .generate_stream("m", "p", false, None, |_| deltas += 1)
            .await
            .expect("stream succeeds after pre-delta retry");
        assert_eq!(text, "chunk");
        assert_eq!(deltas, 1);
        assert_eq!(attempts.load(AtomicOrdering::SeqCst), 2);
    }

    #[tokio::test]
    async fn stream_does_not_retry_after_first_delta() {
        /// Fails mid-stream: emits a delta, then errors. Retry would duplicate
        /// the delta in the UI — must not happen.
        struct FailAfterDelta;
        impl ChatLlmProvider for FailAfterDelta {
            fn generate<'a>(&'a self, _req: &'a ChatRequest) -> ChatFuture<'a> {
                Box::pin(async { Ok(ChatResponse::from_text("ok")) })
            }
            fn generate_stream<'a>(
                &'a self,
                _req: &'a ChatRequest,
                mut on_delta: Box<dyn FnMut(&str) + Send + 'a>,
            ) -> ChatFuture<'a> {
                Box::pin(async move {
                    on_delta("partial");
                    Err(LlmError::new(LlmErrorKind::Timeout, "late failure").into())
                })
            }
            fn capabilities(&self) -> &LlmCapabilities {
                &LLM_CAPS_OPENAI
            }
            fn model_id(&self) -> &str {
                "stub"
            }
        }
        let client = SummaryLlmClient::custom(Box::new(FailAfterDelta), "test");
        let mut deltas = 0u32;
        let err = client
            .generate_stream("m", "p", false, None, |_| deltas += 1)
            .await
            .expect_err("post-delta failure surfaces");
        assert!(format!("{err:#}").contains("late failure"));
        assert_eq!(deltas, 1);
    }

    #[tokio::test]
    async fn soniox_client_fails_at_call_time_with_load_bearing_message() {
        let client = SummaryLlmClient::for_provider(AiProvider::Soniox, "k");
        let err = client
            .generate("m", "p", false, None)
            .await
            .expect_err("soniox has no chat llm");
        assert!(format!("{err:#}").contains("no summary LLM"));
    }

    #[test]
    fn origin_labels_match_legacy_debug_format() {
        let gemini = SummaryLlmClient::for_provider(AiProvider::Gemini, "k");
        assert_eq!(gemini.origin_label(), "Gemini");
        assert_eq!(gemini.builtin_provider(), Some(AiProvider::Gemini));
        let (stub, _) = StubChatLlm::failing(LlmErrorKind::Server, 0);
        let custom = SummaryLlmClient::custom(Box::new(stub), "local");
        assert_eq!(custom.origin_label(), "custom:local");
        assert_eq!(custom.builtin_provider(), None);
        assert!(gemini.supports_summary_language("vi"));
        assert!(!gemini.supports_summary_language("xx"));
        assert!(custom.supports_summary_language("xx"));
    }
}
