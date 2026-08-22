use crate::ai::AiProvider;
use crate::runtime::factories::{generate_summary, test_summary_api_key};

#[tokio::test]
async fn soniox_summary_returns_error_without_network() {
    let err = generate_summary(AiProvider::Soniox, "key", "model", None, "prompt", true)
        .await
        .expect_err("Soniox has no summary LLM");
    let msg = err.to_string();
    assert!(
        msg.contains("Soniox") && msg.contains("summary"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
async fn test_summary_api_key_soniox_does_not_use_live_websocket() {
    let err = test_summary_api_key(AiProvider::Soniox, "key", "model")
        .await
        .expect_err("Soniox has no summary LLM");
    assert!(
        err.contains("Soniox") && err.contains("summary"),
        "unexpected error: {err}"
    );
    assert!(
        !err.contains("WebSocket"),
        "summary key test must not use Live WebSocket: {err}"
    );
}

#[tokio::test]
async fn test_summary_api_key_rejects_empty_key() {
    let err = test_summary_api_key(AiProvider::Gemini, "  ", "gemini-2.5-flash")
        .await
        .expect_err("empty key");
    assert!(err.contains("empty"), "unexpected error: {err}");
}
