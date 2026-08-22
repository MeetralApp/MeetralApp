use crate::meeting::prompts::context::{language_label, session_hint, PromptContext};
use crate::meeting::prompts::render::render;

pub fn build_summary(ctx: &PromptContext<'_>) -> String {
    let lang = language_label(ctx.summary_language);
    let my_lang = language_label(ctx.my_language);
    let meeting_lang = language_label(ctx.meeting_language);
    render(
        include_str!("summary.txt"),
        &[
            ("lang", &lang),
            ("my_lang", &my_lang),
            ("meeting_lang", &meeting_lang),
            ("transcript", ctx.transcript),
            ("session_hint", session_hint(ctx.session_mode)),
            ("title", ctx.meeting_title),
            ("context", ctx.context_block.unwrap_or("")),
        ],
    )
}

pub fn build_merge(ctx: &PromptContext<'_>, partial_notes: &str) -> String {
    let lang = language_label(ctx.summary_language);
    let my_lang = language_label(ctx.my_language);
    let meeting_lang = language_label(ctx.meeting_language);
    render(
        include_str!("merge.txt"),
        &[
            ("lang", &lang),
            ("my_lang", &my_lang),
            ("meeting_lang", &meeting_lang),
            ("notes", partial_notes),
            ("session_hint", session_hint(ctx.session_mode)),
            ("title", ctx.meeting_title),
            ("context", ctx.context_block.unwrap_or("")),
        ],
    )
}
