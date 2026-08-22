use unicode_segmentation::UnicodeSegmentation;

/// Sentence terminators used when scanning for streaming boundaries.
const SENTENCE_TERMINATORS: [char; 7] = ['.', '!', '?', '…', '。', '！', '？'];

const COMMON_ABBREVIATIONS: &[&str] = &[
    "Dr", "Mr", "Mrs", "Ms", "Prof", "Sr", "Jr", "vs", "etc", "e.g", "i.e", "St", "Ave", "Dept",
    "Inc", "Ltd", "Co", "No", "Vol", "Fig", "al", "eg", "ie",
];

fn is_sentence_terminator(c: char) -> bool {
    SENTENCE_TERMINATORS.contains(&c)
}

fn is_closing_char(c: char) -> bool {
    matches!(c, '"' | '\'' | ')' | ']' | '」' | '』' | '»' | '”' | '’')
}

fn word_before_byte_index(text: &str, byte_index: usize) -> &str {
    text[..byte_index]
        .split_whitespace()
        .next_back()
        .unwrap_or("")
}

/// `byte_index` points at `.` — ignore decimals (`3.14`) and common abbreviations (`Dr.`).
fn is_false_period_break(text: &str, period_byte: usize) -> bool {
    let before = &text[..period_byte];
    let after = &text[period_byte + '.'.len_utf8()..];
    if matches!(before.chars().next_back(), Some(c) if c.is_ascii_digit())
        && matches!(after.chars().next(), Some(c) if c.is_ascii_digit())
    {
        return true;
    }
    let word = word_before_byte_index(text, period_byte);
    if word.is_empty() {
        return false;
    }
    if word.chars().count() == 1 && word.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return true;
    }
    COMMON_ABBREVIATIONS
        .iter()
        .any(|abbr| word.eq_ignore_ascii_case(abbr))
}

fn bytes_after_closing_punctuation(text: &str, mut end: usize) -> usize {
    while end < text.len() {
        let ch = text[end..].chars().next().unwrap();
        if is_closing_char(ch) {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn sentence_span_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = Vec::new();
    let mut offset = 0;
    for sentence in text.unicode_sentences() {
        if sentence.is_empty() {
            continue;
        }
        let Some(rel) = text[offset..].find(sentence) else {
            break;
        };
        let start = offset + rel;
        let end = start + sentence.len();
        spans.push(start..end);
        offset = end;
    }
    spans
}

fn sentence_terminator_before_end(text: &str, end: usize) -> Option<(usize, char)> {
    let mut idx = end.min(text.len());
    while idx > 0 {
        let ch = text[..idx].chars().next_back()?;
        if ch.is_whitespace() {
            idx -= ch.len_utf8();
            continue;
        }
        if is_closing_char(ch) {
            idx -= ch.len_utf8();
            continue;
        }
        if is_sentence_terminator(ch) {
            return Some((idx - ch.len_utf8(), ch));
        }
        break;
    }
    None
}

fn boundary_valid_for_natural_commit(text: &str, cut: usize) -> bool {
    if cut == 0 || cut > text.len() {
        return false;
    }
    let tail = text[cut..].trim();
    if tail.is_empty() {
        return false;
    }
    let Some((punct_byte, punct)) = sentence_terminator_before_end(text, cut) else {
        return false;
    };
    if punct == '.' && is_false_period_break(text, punct_byte) {
        return false;
    }
    true
}

fn last_committable_sentence_end_by_scan(text: &str) -> Option<usize> {
    let mut last = None;
    for (i, c) in text.char_indices() {
        if !is_sentence_terminator(c) {
            continue;
        }
        if c == '.' && is_false_period_break(text, i) {
            continue;
        }
        let after_punct = i + c.len_utf8();
        let after_closers = bytes_after_closing_punctuation(text, after_punct);
        match text[after_closers..].chars().next() {
            Some(next)
                if next.is_whitespace() && text[after_closers..].chars().nth(1).is_some() =>
            {
                last = Some(after_closers);
            }
            _ => {}
        }
    }
    last
}

/// Byte index just past the end of the last *complete* sentence in `text`.
///
/// Used by Natural mode (`sentence_boundary` → append + `flush: true`). The tail
/// sentence is treated as still in progress until the provider streams the next one.
pub fn last_committable_sentence_end(text: &str) -> Option<usize> {
    if text.trim().is_empty() {
        return None;
    }

    let spans = sentence_span_ranges(text);
    if spans.len() >= 2 {
        for idx in (0..spans.len() - 1).rev() {
            let cut = spans[idx].end;
            if boundary_valid_for_natural_commit(text, cut) {
                return Some(cut);
            }
        }
    }

    last_committable_sentence_end_by_scan(text)
}

/// Whether trimmed text ends at a sentence terminator (optional closing quotes).
pub fn streaming_text_ends_sentence(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let mut idx = t.len();
    while idx > 0 {
        let Some(ch) = t[..idx].chars().next_back() else {
            return false;
        };
        if is_closing_char(ch) {
            idx -= ch.len_utf8();
            continue;
        }
        return is_sentence_terminator(ch);
    }
    false
}

/// Byte index just past the last internal whitespace, so an over-long clause is
/// cut at a word boundary instead of mid-word. `None` when there is no space.
pub fn last_word_boundary(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|(_, c)| c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .next_back()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_end_requires_terminator_then_space() {
        assert_eq!(last_committable_sentence_end("Hello world."), None);
        let text = "Hello world. Next";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(text[..end].trim_end(), "Hello world.");
    }

    #[test]
    fn sentence_end_picks_last_complete_sentence() {
        let text = "One. Two. Three";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(text[..end].trim_end(), "One. Two.");
    }

    #[test]
    fn sentence_end_handles_ellipsis_and_multibyte() {
        let text = "Xin chào… Tiếp theo";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(&text[..end], "Xin chào…");
    }

    #[test]
    fn sentence_end_skips_abbreviation_period() {
        assert_eq!(last_committable_sentence_end("See Dr. Smith"), None);
        let text = "See Dr. Smith arrived. He waved";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(text[..end].trim_end(), "See Dr. Smith arrived.");
    }

    #[test]
    fn sentence_end_skips_decimal_period() {
        assert_eq!(last_committable_sentence_end("Pi is 3.14"), None);
        let text = "Pi is 3.14 exactly. Next";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(text[..end].trim_end(), "Pi is 3.14 exactly.");
    }

    #[test]
    fn sentence_end_includes_closing_quote_before_next_sentence() {
        let text = "He said \"yes.\" Next";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(text[..end].trim_end(), "He said \"yes.\"");
    }

    #[test]
    fn sentence_end_cjk_terminator_with_following_text() {
        let text = "你好。世界";
        let end = last_committable_sentence_end(text).expect("boundary");
        assert_eq!(&text[..end], "你好。");
    }

    #[test]
    fn streaming_text_ends_sentence_detects_terminators() {
        assert!(streaming_text_ends_sentence("Yes."));
        assert!(streaming_text_ends_sentence("Hello!"));
        assert!(!streaming_text_ends_sentence("Hello"));
    }

    #[test]
    fn word_boundary_cuts_before_partial_word() {
        let text = "alpha beta gam";
        let cut = last_word_boundary(text).expect("space");
        assert_eq!(&text[..cut], "alpha beta ");
    }
}
