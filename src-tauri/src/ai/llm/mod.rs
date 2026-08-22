//! Vendor-agnostic chat-LLM seam. Object-safe trait so `SummaryLlmClient`
//! can wrap `Box<dyn ChatLlmProvider>` and tests can inject deterministic
//! fakes. Provider dispatch stays in `runtime::factories::summary`.

mod client;

pub use client::{
    is_retryable_llm_error, kind_from_http_status, kind_from_reqwest, ChatFuture, ChatLlmProvider,
    ChatRequest, ChatResponse, FailingChatLlm, LlmError, LlmErrorKind, LlmUsage, ToolCallResponse,
    ToolChoice, ToolDefinition,
};
