//! Incremental SSE (Server-Sent Events) parser shared by summary streaming clients.
//!
//! Byte-oriented so multi-byte UTF-8 characters split across chunk boundaries stay
//! intact: lines are only cut at `\n` (ASCII), then decoded independently.

/// Incremental parser for `text/event-stream` bodies.
/// Feed raw byte chunks; completed `data:` payloads come back as strings.
/// `event:` / `id:` / `retry:` fields and comment lines are ignored.
#[derive(Default)]
pub struct SseParser {
    pending: Vec<u8>,
    data_lines: Vec<String>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a raw chunk; returns data payloads of events completed by a blank line.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(nl) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=nl).collect();
            self.consume_line(&line, &mut events);
        }
        events
    }

    /// Flush trailing buffer when the stream ends without a final blank line.
    pub fn finish(&mut self) -> Vec<String> {
        let rest = std::mem::take(&mut self.pending);
        let mut events = Vec::new();
        if !rest.is_empty() {
            self.consume_line(&rest, &mut events);
        }
        if let Some(ev) = self.take_event() {
            events.push(ev);
        }
        events
    }

    fn consume_line(&mut self, raw: &[u8], events: &mut Vec<String>) {
        let line = String::from_utf8_lossy(raw);
        let line = line.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            if let Some(ev) = self.take_event() {
                events.push(ev);
            }
        } else if line.starts_with(':') {
            // comment — keepalive, ignore
        } else if let Some(value) = line.strip_prefix("data:") {
            self.data_lines
                .push(value.strip_prefix(' ').unwrap_or(value).to_string());
        }
    }

    fn take_event(&mut self) -> Option<String> {
        if self.data_lines.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.data_lines).join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_event() {
        let mut p = SseParser::new();
        let events = p.push(b"data: {\"a\":1}\n\n");
        assert_eq!(events, vec!["{\"a\":1}".to_string()]);
        assert!(p.finish().is_empty());
    }

    #[test]
    fn assembles_event_split_across_chunks() {
        let mut p = SseParser::new();
        assert!(p.push(b"data: {\"ans").is_empty());
        assert!(p.push("wer\":\"xin chào\"}\n".as_bytes()).is_empty());
        let events = p.push(b"\n");
        assert_eq!(events, vec!["{\"answer\":\"xin chào\"}".to_string()]);
    }

    #[test]
    fn joins_multi_line_data() {
        let mut p = SseParser::new();
        let events = p.push(b"data: first\ndata: second\n\n");
        assert_eq!(events, vec!["first\nsecond".to_string()]);
    }

    #[test]
    fn handles_crlf_and_comments() {
        let mut p = SseParser::new();
        let events = p.push(b": keepalive\r\ndata: ok\r\n\r\n");
        assert_eq!(events, vec!["ok".to_string()]);
    }

    #[test]
    fn passes_done_marker_through_for_caller() {
        let mut p = SseParser::new();
        let events = p.push(b"data: [DONE]\n\n");
        assert_eq!(events, vec!["[DONE]".to_string()]);
    }

    #[test]
    fn finish_flushes_unterminated_event() {
        let mut p = SseParser::new();
        assert!(p.push(b"data: tail").is_empty());
        assert_eq!(p.finish(), vec!["tail".to_string()]);
    }

    #[test]
    fn ignores_events_without_data() {
        let mut p = SseParser::new();
        let events = p.push(b"event: message\n\ndata: real\n\n");
        assert_eq!(events, vec!["real".to_string()]);
    }
}
