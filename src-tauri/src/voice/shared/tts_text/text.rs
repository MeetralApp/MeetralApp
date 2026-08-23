pub fn normalize_interim_translated(text: &str) -> String {
    text.trim_end().to_string()
}

/// Merge a streaming interim against what we've already tracked.
///
/// Mirrors `meeting::segment_engine::merge_streaming_text` so the custom voice follows
/// the exact same accumulation rule as the on-screen transcript.
pub fn merge_streaming_text(previous: &str, next: &str) -> String {
    if next.trim().is_empty() {
        return previous.to_string();
    }
    if previous.trim().is_empty() {
        return next.trim().to_string();
    }
    let prev = previous.trim();
    let next_trimmed = next.trim();
    if next_trimmed.starts_with(prev) {
        return next_trimmed.to_string();
    }
    if prev.starts_with(next_trimmed) {
        return prev.to_string();
    }
    let needs_space = !prev.ends_with([' ', '-', '(', ','])
        && !prev.ends_with('—')
        && !next_trimmed.starts_with(|c: char| ",.;:!?)".contains(c));
    if needs_space {
        format!("{prev} {next_trimmed}")
    } else {
        format!("{prev}{next_trimmed}")
    }
}

pub fn is_junk_tts_fragment(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed
        .chars()
        .all(|c| c.is_whitespace() || ".,!?…".contains(c))
    {
        return true;
    }
    if trimmed.chars().count() == 1 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junk_fragment_detection() {
        assert!(is_junk_tts_fragment("..."));
        assert!(is_junk_tts_fragment("!"));
        assert!(!is_junk_tts_fragment("Yes"));
        assert!(!is_junk_tts_fragment("OK"));
    }

    #[test]
    fn merge_streaming_appends_gemini_fragments() {
        assert_eq!(merge_streaming_text("", "Xin chào"), "Xin chào");
        assert_eq!(merge_streaming_text("Xin chào", "mọi"), "Xin chào mọi");
        assert_eq!(
            merge_streaming_text("Xin chào mọi", "người,"),
            "Xin chào mọi người,"
        );
    }

    #[test]
    fn merge_streaming_handles_cumulative_openai_text() {
        assert_eq!(merge_streaming_text("Hello", "Hello world"), "Hello world");
        assert_eq!(merge_streaming_text("Hello world", "Hello"), "Hello world");
        assert_eq!(merge_streaming_text("Hello", "Hello"), "Hello");
    }

    #[test]
    fn merge_streaming_no_space_before_punctuation() {
        assert_eq!(merge_streaming_text("Hello", "."), "Hello.");
        assert_eq!(merge_streaming_text("Hello", "world"), "Hello world");
    }
}
