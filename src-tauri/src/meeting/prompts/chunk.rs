//! Prompt + context packing for the summary map step (`chunk.txt`).
//! Builders live next to their template; `build_chunk_extract_prompt` is the
//! one prompt whose `.rs` module was previously folded into `mod.rs` — it now
//! follows the 1 prompt = 1 module convention.

use super::context::{language_label, PromptContext};
use super::render::render;

/// Versioned id for logging/telemetry; bump when the contract changes.
pub const PROMPT_ID: &str = "chunk@v2";

pub fn build_chunk_extract_prompt(ctx: &PromptContext<'_>, transcript_chunk: &str) -> String {
    let lang = language_label(ctx.summary_language);
    let my_lang = language_label(ctx.my_language);
    let meeting_lang = language_label(ctx.meeting_language);
    render(
        include_str!("chunk.txt"),
        &[
            ("lang", &lang),
            ("my_lang", &my_lang),
            ("meeting_lang", &meeting_lang),
            ("transcript", transcript_chunk),
            ("title", ctx.meeting_title),
            ("context", ctx.context_block.unwrap_or("")),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> PromptContext<'static> {
        PromptContext {
            summary_language: "vi",
            my_language: "vi",
            meeting_language: "en",
            transcript: "",
            session_mode: "interpreter",
            meeting_title: "Sprint review",
            context_block: None,
        }
    }

    #[test]
    fn prompt_includes_language_and_transcript_boundary() {
        let prompt = build_chunk_extract_prompt(&ctx(), "[inbound #1] hello");
        assert!(prompt.contains("Vietnamese (vi)"));
        assert!(prompt.contains("English (en)"));
        assert!(prompt.contains("hello"));
        assert!(prompt.contains("Sprint review"));
        assert!(prompt.contains("<transcript>"));
        assert!(prompt.contains("(no notes)"));
    }

    #[test]
    fn prompt_renders_context_block_when_present() {
        let with_ctx = PromptContext {
            context_block: Some(
                "Meeting context (trusted, user-provided):\n- Domain: Software development",
            ),
            ..ctx()
        };
        let prompt = build_chunk_extract_prompt(&with_ctx, "x");
        assert!(prompt.contains("- Domain: Software development"));
        assert!(prompt.contains("Terminology"));
    }

    #[test]
    fn renders_language_as_label() {
        let prompt = build_chunk_extract_prompt(
            &PromptContext {
                summary_language: "ja",
                ..ctx()
            },
            "x",
        );
        assert!(prompt.contains("Japanese (ja)"));
    }
}
