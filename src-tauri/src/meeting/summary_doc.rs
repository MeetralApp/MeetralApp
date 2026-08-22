//! TipTap document helpers for meeting summaries.
//! `generated_json` stores TipTap JSON; `generated_text` is derived export prose.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use super::models::TranscriptSegmentView;
use super::summary_schema::{
    block_points, block_title, MeetingBriefSummary, SummaryBlockKind, SUMMARY_BLOCKS,
};

/// Build a TipTap doc from a parsed meeting brief + transcript segments.
pub fn seed_from_brief(
    summary: &MeetingBriefSummary,
    segments: &[TranscriptSegmentView],
    summary_language: &str,
) -> Value {
    let mut by_dir_seq: HashMap<(String, i32), &TranscriptSegmentView> =
        HashMap::with_capacity(segments.len());
    for seg in segments {
        by_dir_seq.insert((seg.direction.clone(), seg.sequence), seg);
    }

    let mut content = Vec::new();
    for block_kind in SUMMARY_BLOCKS {
        let points = block_points(summary, block_kind);
        if points.is_empty() {
            continue;
        }
        content.push(json!({
            "type": "heading",
            "attrs": { "level": 3 },
            "content": [{ "type": "text", "text": block_title(block_kind, summary_language) }]
        }));

        let mut items = Vec::with_capacity(points.len());
        for point in points {
            let mut cites = Vec::new();
            let mut seen = HashSet::new();
            for seg_ref in &point.segment_refs {
                let key = (seg_ref.direction.clone(), seg_ref.sequence);
                let Some(seg) = by_dir_seq.get(&key) else {
                    continue;
                };
                if !seen.insert(seg.id.clone()) {
                    continue;
                }
                cites.push(citation_node(seg));
            }

            let mut meta_parts = Vec::new();
            if block_kind == SummaryBlockKind::ActionItems {
                if let Some(owner) = point
                    .owner
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    meta_parts.push(format!("Owner: {owner}"));
                }
                if let Some(due) = point
                    .due
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    meta_parts.push(format!("Due: {due}"));
                }
            }
            let meta = if meta_parts.is_empty() {
                None
            } else {
                Some(meta_parts.join(" · "))
            };
            items.push(json!({
                "type": "listItem",
                "content": [point_paragraph(&point.text, &cites, meta.as_deref())]
            }));
        }
        content.push(json!({ "type": "bulletList", "content": items }));
    }

    if content.is_empty() {
        return json!({ "type": "doc", "content": [{ "type": "paragraph" }] });
    }
    json!({ "type": "doc", "content": content })
}

fn citation_node(seg: &TranscriptSegmentView) -> Value {
    json!({
        "type": "citation",
        "attrs": {
            "segmentId": seg.id,
            "direction": seg.direction,
            "startedAtMs": seg.started_at_ms,
        }
    })
}

fn point_paragraph(text: &str, cites: &[Value], meta: Option<&str>) -> Value {
    let mut content = Vec::new();
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        content.push(json!({ "type": "text", "text": trimmed }));
    }
    if let Some(meta) = meta {
        if content.is_empty() {
            content.push(json!({ "type": "text", "text": meta }));
        } else {
            content.push(json!({ "type": "text", "text": format!(" ({meta})") }));
        }
    }
    for cite in cites {
        if !content.is_empty() {
            content.push(json!({ "type": "text", "text": " " }));
        }
        content.push(cite.clone());
    }
    if content.is_empty() {
        json!({ "type": "paragraph" })
    } else {
        json!({ "type": "paragraph", "content": content })
    }
}

/// TipTap JSON → export prose; citation atoms become `3:48·You`.
pub fn doc_to_prose(doc: &Value) -> String {
    let mut lines = Vec::new();
    walk_block(doc, &mut lines, "");
    lines.join("\n").trim().to_string()
}

fn walk_block(node: &Value, lines: &mut Vec<String>, prefix: &str) {
    let Some(obj) = node.as_object() else {
        if let Some(arr) = node.as_array() {
            for child in arr {
                walk_block(child, lines, prefix);
            }
        }
        return;
    };
    let ty = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match ty {
        "doc" => {
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    walk_block(child, lines, prefix);
                }
            }
        }
        "heading" => {
            let level = obj
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(3) as usize;
            let text = inline_to_text(node).trim().to_string();
            if !text.is_empty() {
                lines.push(format!("{} {}", "#".repeat(level.max(1)), text));
            }
            lines.push(String::new());
        }
        "paragraph" => {
            let text = inline_to_text(node).trim().to_string();
            if !text.is_empty() {
                lines.push(format!("{prefix}{text}"));
            }
        }
        "bulletList" | "orderedList" => {
            let ordered = ty == "orderedList";
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for item in content {
                    walk_list_item(item, lines, ordered);
                }
            }
            lines.push(String::new());
        }
        "blockquote" => {
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    walk_block(child, lines, "> ");
                }
            }
        }
        _ => {
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    walk_block(child, lines, prefix);
                }
            }
        }
    }
}

fn walk_list_item(node: &Value, lines: &mut Vec<String>, ordered: bool) {
    let marker = if ordered { "1. " } else { "- " };
    let Some(content) = node
        .as_object()
        .and_then(|o| o.get("content"))
        .and_then(|c| c.as_array())
    else {
        lines.push(marker.to_string());
        return;
    };
    if content.is_empty() {
        lines.push(marker.to_string());
        return;
    }
    for (i, child) in content.iter().enumerate() {
        let ty = child
            .as_object()
            .and_then(|o| o.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        if ty == "paragraph" {
            let text = inline_to_text(child).trim().to_string();
            if i == 0 {
                lines.push(format!("{marker}{text}"));
            } else {
                lines.push(format!("  {text}"));
            }
        } else {
            walk_block(child, lines, "  ");
        }
    }
}

fn inline_to_text(node: &Value) -> String {
    let Some(obj) = node.as_object() else {
        return String::new();
    };
    let ty = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match ty {
        "text" => obj
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string(),
        "hardBreak" => "\n".to_string(),
        "citation" => {
            let attrs = obj.get("attrs");
            let direction = attrs
                .and_then(|a| a.get("direction"))
                .and_then(|d| d.as_str())
                .unwrap_or("outbound");
            let started_at_ms = attrs
                .and_then(|a| a.get("startedAtMs"))
                .and_then(|s| s.as_i64())
                .unwrap_or(0);
            citation_label(direction, started_at_ms)
        }
        _ => {
            let mut out = String::new();
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    out.push_str(&inline_to_text(child));
                }
            }
            out
        }
    }
}

fn citation_label(direction: &str, started_at_ms: i64) -> String {
    let timer = format_segment_timestamp(started_at_ms);
    let speaker = if direction == "outbound" {
        "You"
    } else {
        "Meeting"
    };
    format!("{timer}·{speaker}")
}

fn format_segment_timestamp(started_at_ms: i64) -> String {
    let total_sec = (started_at_ms / 1000).max(0);
    let hours = total_sec / 3600;
    let minutes = (total_sec % 3600) / 60;
    let seconds = total_sec % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Walk TipTap JSON for `citation` nodes; return unique segmentIds in document order.
pub fn collect_citation_segment_ids(node: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    walk_citations(node, &mut out, &mut seen);
    out
}

fn walk_citations(node: &Value, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    if let Some(obj) = node.as_object() {
        if obj.get("type").and_then(|t| t.as_str()) == Some("citation") {
            if let Some(id) = obj
                .get("attrs")
                .and_then(|a| a.get("segmentId"))
                .and_then(|s| s.as_str())
            {
                let id = id.trim();
                if !id.is_empty() && seen.insert(id.to_string()) {
                    out.push(id.to_string());
                }
            }
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
            for child in content {
                walk_citations(child, out, seen);
            }
        }
    } else if let Some(arr) = node.as_array() {
        for child in arr {
            walk_citations(child, out, seen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::summary_schema::{SegmentRef, SummaryPoint, SummaryPointBlock};

    fn sample_brief() -> MeetingBriefSummary {
        MeetingBriefSummary {
            schema_version: 3,
            overview: SummaryPointBlock {
                points: vec![SummaryPoint {
                    id: "ov-1".into(),
                    text: "Hello".into(),
                    owner: None,
                    due: None,
                    segment_refs: vec![SegmentRef {
                        direction: "outbound".into(),
                        sequence: 0,
                    }],
                    supersedes_index: None,
                }],
            },
            key_points: SummaryPointBlock::default(),
            decisions: SummaryPointBlock::default(),
            action_items: SummaryPointBlock::default(),
            open_questions: SummaryPointBlock::default(),
            entities: Vec::new(),
            yesterday: SummaryPointBlock::default(),
            today: SummaryPointBlock::default(),
            blockers: SummaryPointBlock::default(),
        }
    }

    #[test]
    fn seed_embeds_citation_atoms() {
        let seg = TranscriptSegmentView {
            id: "s1".into(),
            meeting_id: "m1".into(),
            direction: "outbound".into(),
            sequence: 0,
            source_text: "a".into(),
            translated_text: "b".into(),
            started_at_ms: 199_000,
            ended_at_ms: 200_000,
            connection_gap: false,
        };
        let doc = seed_from_brief(&sample_brief(), &[seg], "en");
        let cites = collect_citation_segment_ids(&doc);
        assert_eq!(cites, vec!["s1".to_string()]);
        let prose = doc_to_prose(&doc);
        assert!(prose.contains("### Overview"));
        assert!(prose.contains("Hello"));
        assert!(prose.contains("3:19·You"));
    }

    #[test]
    fn collect_citation_segment_ids_dedupes_in_order() {
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Hi " },
                    { "type": "citation", "attrs": { "segmentId": "s1", "direction": "outbound", "startedAtMs": 0 } },
                    { "type": "citation", "attrs": { "segmentId": "s2", "direction": "inbound", "startedAtMs": 1000 } },
                    { "type": "citation", "attrs": { "segmentId": "s1", "direction": "outbound", "startedAtMs": 0 } }
                ]
            }]
        });
        assert_eq!(
            collect_citation_segment_ids(&doc),
            vec!["s1".to_string(), "s2".to_string()]
        );
    }
}
