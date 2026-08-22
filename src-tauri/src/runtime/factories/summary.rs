//! Summary LLM factory — provider dispatch for meeting summaries.
//!
//! To add a summary vendor:
//! 1. `providers/<name>/summary.rs` with a `ChatLlmProvider` impl
//! 2. Match arm here in `chat_llm_for`
//! 3. `supports_native_summary: true` + `summary_models` in `capabilities/catalog.rs`
//! 4. Arm in `AppConfig::summary_provider_and_key`
//! 5. FE summary settings / API key field if needed
//!
//! Domain prompts stay in `meeting/prompts/` — never inside provider crates.

use anyhow::Result;

use crate::ai::llm::{ChatLlmProvider, ChatRequest};
use crate::ai::provider::AiProvider;
use crate::ai::SummaryLlmClient;
use crate::config::LlmSelection;
use crate::providers::{compatible, gemini, openai};

/// The only `match AiProvider` site for chat-LLM construction (boundaries
/// rule). Soniox has no chat LLM — the error message is load-bearing (tests
/// assert on it).
pub fn chat_llm_for(
    provider: AiProvider,
    api_key: &str,
    model: &str,
) -> Result<Box<dyn ChatLlmProvider>> {
    match provider {
        AiProvider::Gemini => Ok(Box::new(gemini::summary::GeminiChatLlm::new(
            api_key, model,
        ))),
        AiProvider::OpenAi => Ok(Box::new(openai::summary::OpenAiChatLlm::new(
            api_key, model,
        ))),
        AiProvider::Soniox => Err(anyhow::anyhow!(
            "Soniox has no summary LLM. Add a Gemini or OpenAI API key."
        )),
    }
}

/// Build the retry/observability wrapper for a resolved selection (built-in
/// or custom profile). Model resolution already happened in
/// `AppConfig::summary_llm_selection` (config zone).
pub fn summary_llm_client_for_selection(selection: &LlmSelection) -> Result<SummaryLlmClient> {
    match selection {
        LlmSelection::BuiltIn {
            provider, api_key, ..
        } => Ok(SummaryLlmClient::for_provider(*provider, api_key)),
        LlmSelection::Custom { profile, api_key } => Ok(SummaryLlmClient::custom(
            Box::new(compatible::chat::CompatibleChatLlm::new(
                &profile.base_url,
                api_key.as_deref(),
                &profile.chat_model,
                profile.json_mode,
            )),
            &profile.id,
        )),
    }
}

/// Generate a summary via the selected provider's REST LLM.
pub async fn generate_summary(
    provider: AiProvider,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
) -> Result<String> {
    let client = chat_llm_for(provider, api_key, model)?;
    Ok(client
        .generate(&ChatRequest::plain(model, system, prompt, json_response))
        .await?
        .text)
}

/// Streaming summary via the selected provider. `on_delta` fires per text chunk.
pub async fn generate_summary_stream(
    provider: AiProvider,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    prompt: &str,
    json_response: bool,
    on_delta: impl FnMut(&str) + Send,
) -> Result<String> {
    let client = chat_llm_for(provider, api_key, model)?;
    Ok(client
        .generate_stream(
            &ChatRequest::plain(model, system, prompt, json_response),
            Box::new(on_delta),
        )
        .await?
        .text)
}

/// Validate a summary-provider API key with a tiny REST generate (not Live WebSocket).
pub async fn test_summary_api_key(
    provider: AiProvider,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("API key is empty".into());
    }
    generate_summary(
        provider,
        api_key,
        model,
        None,
        "Reply with the single word OK.",
        false,
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}
