//! OpenAI-compatible vendor slice — ONE implementation covers
//! the whole local-server ecosystem (Ollama, LM Studio, llama.cpp server,
//! vLLM). Profiles are data (`config::custom_llm::CustomLlmProfile`); this
//! module is the wire protocol only.

pub mod chat;

/// Join a profile base URL with an endpoint path. Users paste the base in
/// both conventions — bare host (`http://localhost:11434`) and SDK-style
/// with the API prefix (`http://localhost:20128/v1`) — so accept both:
/// a trailing `/v1` is detected and not duplicated.
pub(crate) fn endpoint_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.to_ascii_lowercase().ends_with("/v1") {
        format!("{base}/{path}")
    } else {
        format!("{base}/v1/{path}")
    }
}

/// First characters of a response body, for parse-error diagnostics. A
/// non-JSON body (HTML error page, empty 200, proxy banner) is otherwise
/// invisible in logs — the excerpt makes the mismatch obvious.
pub(crate) fn body_excerpt(body: &str) -> String {
    const MAX: usize = 160;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "<empty body>".to_string();
    }
    if trimmed.len() <= MAX {
        trimmed.to_string()
    } else {
        format!("{}…", &trimmed[..MAX])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_url_accepts_both_base_conventions() {
        assert_eq!(
            endpoint_url("http://localhost:11434", "chat/completions"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url("http://localhost:11434/", "chat/completions"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url("http://localhost:20128/v1", "chat/completions"),
            "http://localhost:20128/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url("http://localhost:20128/v1/", "chat/completions"),
            "http://localhost:20128/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url("http://localhost:1234/V1", "chat/completions"),
            "http://localhost:1234/V1/chat/completions"
        );
    }

    #[test]
    fn body_excerpt_handles_empty_and_long() {
        assert_eq!(body_excerpt("   "), "<empty body>");
        assert_eq!(body_excerpt("not json"), "not json");
        let long = "x".repeat(300);
        let excerpt = body_excerpt(&long);
        assert!(excerpt.ends_with('…') && excerpt.chars().count() == 161);
    }
}
