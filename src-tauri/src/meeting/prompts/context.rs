//! Prompt context shared by summary / merge builders.

#[derive(Clone, Copy)]
pub struct PromptContext<'a> {
    pub summary_language: &'a str,
    pub my_language: &'a str,
    pub meeting_language: &'a str,
    pub transcript: &'a str,
    /// `"notes"` | `"interpreter"` (and future modes).
    pub session_mode: &'a str,
    /// Meeting display title — grounding hint for the summarizer.
    pub meeting_title: &'a str,
    /// Rendered user-provided meeting context block, already budgeted
    /// by [`render_summary_context_block`]. `None`/empty → prompts render
    /// exactly as before (zero regression when no context is configured).
    pub context_block: Option<&'a str>,
}

pub fn session_hint(session_mode: &str) -> &'static str {
    if session_mode.eq_ignore_ascii_case("notes") {
        "Notes mode: transcript is primarily same-language capture (STT). Prefer faithful notes over cross-language synthesis."
    } else {
        "Interpreter mode: transcript has bilingual columns (You / Meeting). Synthesize across both directions when available."
    }
}

/// Prompt budget for the rendered meeting-context block on summary/merge
/// prompts. Well under the ~8k-token context window guidance.
pub const SUMMARY_CONTEXT_CHAR_BUDGET: usize = 2_000;
/// Smaller budget for the map-step chunk prompt (context repeats per chunk).
pub const CHUNK_CONTEXT_CHAR_BUDGET: usize = 1_000;

/// Renders the user-provided meeting context for **summary** prompts.
/// Includes terminology pairs and terms — model IS allowed to correct
/// live-translation mistakes (e.g., enforce exact spelling/casing).
pub fn render_summary_context_block(
    payload: &crate::config::MeetingContextPayload,
    budget: usize,
) -> Option<String> {
    render_context_block(payload, budget)
}

fn render_context_block(
    payload: &crate::config::MeetingContextPayload,
    budget: usize,
) -> Option<String> {
    if payload.is_empty() {
        return None;
    }
    let mut block = String::new();
    for pair in &payload.general {
        let key = pair.key.trim();
        let value = pair.value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let label = {
            let mut chars = key.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => key.to_string(),
            }
        };
        block.push_str(&format!("- {label}: {value}\n"));
    }

    let terms: Vec<&str> = payload
        .terms
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();
    if !terms.is_empty() {
        block.push_str(&format!(
            "Terms (preserve exact spelling/casing): {}\n",
            terms.join(", ")
        ));
    }
    let pairs: Vec<String> = payload
        .translation_terms
        .iter()
        .filter_map(|t| {
            let source = t.source.trim();
            let target = t.target.trim();
            if source.is_empty() || target.is_empty() {
                None
            } else {
                Some(format!("{source} → {target}"))
            }
        })
        .collect();
    if !pairs.is_empty() {
        block.push_str(&format!(
            "Terminology (always use these exact forms): {}\n",
            pairs.join("; ")
        ));
    }

    let text = payload.text.trim();
    if !text.is_empty() {
        block.push_str(&format!("Background: {text}\n"));
    }
    let body = block.trim_end();
    if body.is_empty() {
        return None;
    }
    let header = "Meeting context (trusted, user-provided):\n";
    let with_header = format!("{header}{body}");
    let char_count = with_header.chars().count();
    if char_count <= budget {
        return Some(with_header);
    }
    // Over budget: drop the Background section first, then truncate
    let mut head = String::from(header);
    for line in body.lines() {
        if line.starts_with("Background: ") {
            continue;
        }
        head.push_str(line);
        head.push('\n');
    }
    let head = head.trim_end().to_string();
    if head == header.trim_end() {
        return None;
    }
    let head_chars = head.chars().count();
    if head_chars <= budget {
        return Some(head);
    }
    let keep = budget.saturating_sub(1);
    let byte_end = head
        .char_indices()
        .nth(keep)
        .map(|(idx, _)| idx)
        .unwrap_or(head.len());
    Some(format!("{}…", head[..byte_end].trim_end()))
}

/// ISO 639-1 code → English display name, for building human-readable language
/// labels used in prompt injection (`"vi"` → `"Vietnamese (vi)"`).
const LANGUAGE_NAMES: &[(&str, &str)] = &[
    ("af", "Afrikaans"),
    ("sq", "Albanian"),
    ("ar", "Arabic"),
    ("az", "Azerbaijani"),
    ("eu", "Basque"),
    ("be", "Belarusian"),
    ("bn", "Bengali"),
    ("bs", "Bosnian"),
    ("bg", "Bulgarian"),
    ("ca", "Catalan"),
    ("zh", "Chinese"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("et", "Estonian"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("gl", "Galician"),
    ("de", "German"),
    ("el", "Greek"),
    ("gu", "Gujarati"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("ko", "Korean"),
    ("lv", "Latvian"),
    ("lt", "Lithuanian"),
    ("mk", "Macedonian"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mr", "Marathi"),
    ("no", "Norwegian"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("pa", "Punjabi"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sr", "Serbian"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("es", "Spanish"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tl", "Tagalog"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
];

/// Human-readable language label for prompt injection: `"vi"` → `"Vietnamese (vi)"`.
/// Models follow full language names more reliably than bare ISO codes.
/// Values that are not a known ISO code (e.g. an already-human name) pass
/// through unchanged.
pub fn language_label(code_or_name: &str) -> String {
    let primary = crate::ai::language_flags::primary_language_code(code_or_name);
    LANGUAGE_NAMES
        .iter()
        .find(|(code, _)| *code == primary)
        .map(|(code, name)| format!("{name} ({code})"))
        .unwrap_or_else(|| code_or_name.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MeetingContextPair, MeetingContextPayload, MeetingContextTranslationTerm};

    fn payload() -> MeetingContextPayload {
        MeetingContextPayload {
            general: vec![
                MeetingContextPair {
                    key: "domain".into(),
                    value: "Software development".into(),
                },
                MeetingContextPair {
                    key: "organization".into(),
                    value: "Acme Corp".into(),
                },
            ],
            text: "Q3 planning for the mobile team.".into(),
            terms: vec!["Sprint".into(), "SOC2".into()],
            translation_terms: vec![MeetingContextTranslationTerm {
                source: "Sprint planning".into(),
                target: "Sprint planning".into(),
            }],
        }
    }

    #[test]
    fn empty_payload_renders_none() {
        assert!(render_summary_context_block(
            &MeetingContextPayload::default(),
            SUMMARY_CONTEXT_CHAR_BUDGET
        )
        .is_none());
        // Whitespace-only fields count as empty too.
        let blank = MeetingContextPayload {
            general: vec![MeetingContextPair {
                key: "  ".into(),
                value: " ".into(),
            }],
            text: "  ".into(),
            terms: vec![" ".into()],
            translation_terms: vec![MeetingContextTranslationTerm {
                source: String::new(),
                target: String::new(),
            }],
        };
        assert!(render_summary_context_block(&blank, SUMMARY_CONTEXT_CHAR_BUDGET).is_none());
    }

    #[test]
    fn renders_all_sections_in_priority_order() {
        let block = render_summary_context_block(&payload(), SUMMARY_CONTEXT_CHAR_BUDGET).unwrap();
        let domain = block.find("- Domain: Software development").unwrap();
        let org = block.find("- Organization: Acme Corp").unwrap();
        let terms = block
            .find("Terms (preserve exact spelling/casing): Sprint, SOC2")
            .unwrap();
        let terminology = block
            .find("Terminology (always use these exact forms): Sprint planning → Sprint planning")
            .unwrap();
        let background = block
            .find("Background: Q3 planning for the mobile team.")
            .unwrap();
        assert!(domain < org && org < terms && terms < terminology && terminology < background);
    }

    #[test]
    fn skips_blank_pairs_but_keeps_section_content() {
        let mut p = payload();
        p.general.push(MeetingContextPair {
            key: "topic".into(),
            value: "  ".into(),
        });
        let block = render_summary_context_block(&p, SUMMARY_CONTEXT_CHAR_BUDGET).unwrap();
        assert!(!block.contains("topic"));
        assert!(block.contains("- Domain: Software development"));
    }

    #[test]
    fn over_budget_drops_background_first() {
        let mut p = payload();
        p.text = "x".repeat(SUMMARY_CONTEXT_CHAR_BUDGET * 2);
        let block = render_summary_context_block(&p, SUMMARY_CONTEXT_CHAR_BUDGET).unwrap();
        assert!(!block.contains("Background:"));
        assert!(block.contains("- Domain: Software development"));
        assert!(block.chars().count() <= SUMMARY_CONTEXT_CHAR_BUDGET);
    }

    #[test]
    fn over_budget_truncates_on_character_boundary_with_ellipsis() {
        let p = MeetingContextPayload {
            general: vec![MeetingContextPair {
                key: "domain".into(),
                value: "Công nghệ phần mềm — rất dài ".repeat(50),
            }],
            ..Default::default()
        };
        let block = render_summary_context_block(&p, 200).unwrap();
        assert!(block.ends_with('…'));
        // Budget 200 chars, keep at most 199 + ellipsis = 200 chars.
        assert!(block.chars().count() <= 200);
    }

    #[test]
    fn over_budget_all_background_only_returns_none() {
        let p = MeetingContextPayload {
            text: "x".repeat(SUMMARY_CONTEXT_CHAR_BUDGET * 2),
            ..Default::default()
        };
        assert!(render_summary_context_block(&p, SUMMARY_CONTEXT_CHAR_BUDGET).is_none());
    }
}
