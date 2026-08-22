mod client;

pub use client::SummaryLlmClient;

pub const SUMMARY_LLM_TIMEOUT_SECS: u64 = 300;

/// Max silence between chunks on a streaming summary call before bailing.
pub const SUMMARY_STREAM_IDLE_TIMEOUT_SECS: u64 = 30;
