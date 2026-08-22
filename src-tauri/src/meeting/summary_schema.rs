use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::prompts::DEFAULT_SUMMARY_TEMPLATE_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryBlockKind {
    Overview,
    KeyPoints,
    Decisions,
    ActionItems,
    OpenQuestions,
    Yesterday,
    Today,
    Blockers,
}

pub const SUMMARY_BLOCKS: [SummaryBlockKind; 8] = [
    SummaryBlockKind::Overview,
    SummaryBlockKind::KeyPoints,
    SummaryBlockKind::Decisions,
    SummaryBlockKind::ActionItems,
    SummaryBlockKind::OpenQuestions,
    SummaryBlockKind::Yesterday,
    SummaryBlockKind::Today,
    SummaryBlockKind::Blockers,
];

/// Schema v4 adds optional, same-pass decision supersession references.
pub const MEETING_BRIEF_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRef {
    pub direction: String,
    pub sequence: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPoint {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, rename = "segmentRefs")]
    pub segment_refs: Vec<SegmentRef>,
    /// One-based index of an earlier decision in the same summary pass.
    /// This is resolved to a stable artifact id while artifacts are regenerated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPointBlock {
    #[serde(default)]
    pub points: Vec<SummaryPoint>,
}

/// Entity kinds the summary prompt may emit; anything else normalizes to generic.
pub const ENTITY_KINDS: [&str; 5] = ["ticket", "system", "person", "product", "generic"];

/// One explicitly-mentioned entity (schema v3 additive block).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SummaryEntity {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_entity_kind")]
    pub kind: String,
    #[serde(default, rename = "segmentRefs")]
    pub segment_refs: Vec<SegmentRef>,
}

fn default_entity_kind() -> String {
    "generic".to_string()
}

/// Meeting brief schema (LLM output only — persisted as TipTap in `generated_json`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MeetingBriefSummary {
    pub schema_version: u32,
    pub overview: SummaryPointBlock,
    pub key_points: SummaryPointBlock,
    pub decisions: SummaryPointBlock,
    pub action_items: SummaryPointBlock,
    #[serde(default, skip_serializing_if = "SummaryPointBlock::is_empty")]
    pub open_questions: SummaryPointBlock,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<SummaryEntity>,
    #[serde(default, skip_serializing_if = "SummaryPointBlock::is_empty")]
    pub yesterday: SummaryPointBlock,
    #[serde(default, skip_serializing_if = "SummaryPointBlock::is_empty")]
    pub today: SummaryPointBlock,
    #[serde(default, skip_serializing_if = "SummaryPointBlock::is_empty")]
    pub blockers: SummaryPointBlock,
}

impl SummaryPointBlock {
    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

pub fn block_points(
    summary: &MeetingBriefSummary,
    block_kind: SummaryBlockKind,
) -> &[SummaryPoint] {
    match block_kind {
        SummaryBlockKind::Overview => &summary.overview.points,
        SummaryBlockKind::KeyPoints => &summary.key_points.points,
        SummaryBlockKind::Decisions => &summary.decisions.points,
        SummaryBlockKind::ActionItems => &summary.action_items.points,
        SummaryBlockKind::OpenQuestions => &summary.open_questions.points,
        SummaryBlockKind::Yesterday => &summary.yesterday.points,
        SummaryBlockKind::Today => &summary.today.points,
        SummaryBlockKind::Blockers => &summary.blockers.points,
    }
}

pub fn block_title(block_kind: SummaryBlockKind, lang: &str) -> &'static str {
    match (lang, block_kind) {
        ("vi", SummaryBlockKind::Overview) => "Tổng quan",
        ("vi", SummaryBlockKind::KeyPoints) => "Những điểm quan trọng",
        ("vi", SummaryBlockKind::Decisions) => "Quyết định",
        ("vi", SummaryBlockKind::ActionItems) => "Việc cần làm",
        ("vi", SummaryBlockKind::OpenQuestions) => "Câu hỏi còn mở",
        ("vi", SummaryBlockKind::Yesterday) => "Hôm qua đã làm",
        ("vi", SummaryBlockKind::Today) => "Hôm nay sẽ làm",
        ("vi", SummaryBlockKind::Blockers) => "Cản trở",
        (_, SummaryBlockKind::Overview) => "Overview",
        (_, SummaryBlockKind::KeyPoints) => "Key points",
        (_, SummaryBlockKind::Decisions) => "Decisions",
        (_, SummaryBlockKind::ActionItems) => "Action items",
        (_, SummaryBlockKind::OpenQuestions) => "Open questions",
        (_, SummaryBlockKind::Yesterday) => "Yesterday",
        (_, SummaryBlockKind::Today) => "Today",
        (_, SummaryBlockKind::Blockers) => "Blockers",
    }
}

pub fn parse_meeting_brief(json: &str) -> Result<MeetingBriefSummary> {
    let value: Value = serde_json::from_str(json).context("parse meeting brief summary JSON")?;
    let mut summary = meeting_brief_from_value(value)?;
    normalize_block(&mut summary.overview.points, "ov");
    normalize_block(&mut summary.key_points.points, "kp");
    normalize_block(&mut summary.decisions.points, "de");
    normalize_block(&mut summary.action_items.points, "ai");
    normalize_block(&mut summary.open_questions.points, "oq");
    normalize_block(&mut summary.yesterday.points, "ye");
    normalize_block(&mut summary.today.points, "to");
    normalize_block(&mut summary.blockers.points, "bl");
    normalize_entities(&mut summary.entities);
    summary.schema_version = MEETING_BRIEF_SCHEMA_VERSION;
    Ok(summary)
}

fn meeting_brief_from_value(value: Value) -> Result<MeetingBriefSummary> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("summary JSON must be an object"))?;

    let overview = block_from_value(obj.get("overview"))?;
    let key_points = block_from_value(obj.get("keyPoints").or_else(|| obj.get("key_points")))?;
    let decisions = block_from_value(obj.get("decisions")).unwrap_or_default();
    let open_questions = block_from_value(
        obj.get("openQuestions")
            .or_else(|| obj.get("open_questions")),
    )
    .unwrap_or_default();
    let action_items = block_from_value(obj.get("actionItems").or_else(|| obj.get("action_items")))
        .unwrap_or_default();
    // Schema v3 additive — v2 briefs parse with an empty list.
    let entities: Vec<SummaryEntity> = obj
        .get("entities")
        .map(|value| serde_json::from_value(value.clone()).context("parse entities"))
        .transpose()?
        .unwrap_or_default();
    // Schema v4 additive — standup blocks; v4 briefs parse with empty arrays.
    let yesterday = block_from_value(obj.get("yesterday")).unwrap_or_default();
    let today = block_from_value(obj.get("today")).unwrap_or_default();
    let blockers = block_from_value(obj.get("blockers")).unwrap_or_default();

    Ok(MeetingBriefSummary {
        schema_version: MEETING_BRIEF_SCHEMA_VERSION,
        overview,
        key_points,
        decisions,
        action_items,
        open_questions,
        entities,
        yesterday,
        today,
        blockers,
    })
}

/// Trim names, drop empties, coerce unknown kinds to `generic`, dedupe by name.
fn normalize_entities(entities: &mut Vec<SummaryEntity>) {
    let mut seen = std::collections::HashSet::new();
    entities.retain_mut(|entity| {
        let name = entity.name.trim();
        if name.is_empty() || !seen.insert(name.to_string()) {
            return false;
        }
        entity.name = name.to_string();
        let kind = entity.kind.trim().to_ascii_lowercase();
        entity.kind = if ENTITY_KINDS.contains(&kind.as_str()) {
            kind
        } else {
            "generic".to_string()
        };
        true
    });
}

fn block_from_value(value: Option<&Value>) -> Result<SummaryPointBlock> {
    let Some(value) = value else {
        return Ok(SummaryPointBlock::default());
    };
    serde_json::from_value(value.clone()).context("parse summary point block")
}

fn normalize_block(points: &mut [SummaryPoint], prefix: &str) {
    for (index, point) in points.iter_mut().enumerate() {
        if point.id.trim().is_empty() {
            point.id = format!("{prefix}-{}", index + 1);
        }
        if point.text.trim().is_empty() {
            point.text = "—".to_string();
        }
        if let Some(owner) = point.owner.as_mut() {
            let trimmed = owner.trim();
            if trimmed.is_empty() {
                point.owner = None;
            } else {
                *owner = trimmed.to_string();
            }
        }
        if let Some(due) = point.due.as_mut() {
            let trimmed = due.trim();
            if trimmed.is_empty() {
                point.due = None;
            } else {
                *due = trimmed.to_string();
            }
        }
        if point.supersedes_index == Some(0) {
            point.supersedes_index = None;
        }
    }
}

pub fn parse_summary_json(template_id: &str, json: &str) -> Result<MeetingBriefSummary> {
    match template_id {
        DEFAULT_SUMMARY_TEMPLATE_ID
        | "executive_summary"
        | "action_items_only"
        | "decisions_log"
        | "standup" => parse_meeting_brief(json),
        _ => anyhow::bail!("Unknown summary template: {template_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v2_with_owner_due() {
        let json = r#"{
            "schemaVersion": 2,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "decisions": { "points": [{ "id": "de-1", "text": "Go ahead", "segmentRefs": [] }] },
          "actionItems": {
            "points": [{
              "id": "ai-1",
              "text": "Ship it",
              "owner": "Nhan",
              "due": "Friday",
              "segmentRefs": [{ "direction": "outbound", "sequence": 3 }]
            }]
          },
          "openQuestions": { "points": [] }
        }"#;
        let parsed = parse_meeting_brief(json).unwrap();
        assert_eq!(parsed.decisions.points[0].text, "Go ahead");
        assert_eq!(parsed.action_items.points[0].owner.as_deref(), Some("Nhan"));
        assert_eq!(parsed.action_items.points[0].due.as_deref(), Some("Friday"));
        assert_eq!(
            block_points(&parsed, SummaryBlockKind::ActionItems)[0].segment_refs[0].sequence,
            3
        );
    }

    #[test]
    fn rejects_non_object() {
        assert!(parse_meeting_brief("[]").is_err());
    }

    #[test]
    fn v2_brief_without_entities_parses_with_empty_entities() {
        let json = r#"{
          "schemaVersion": 2,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "decisions": { "points": [] },
          "actionItems": { "points": [] },
          "openQuestions": { "points": [] }
        }"#;
        let parsed = parse_meeting_brief(json).unwrap();
        assert!(parsed.entities.is_empty());
        assert_eq!(parsed.schema_version, MEETING_BRIEF_SCHEMA_VERSION);
    }

    #[test]
    fn parses_v3_entities_and_normalizes() {
        let json = r#"{
          "schemaVersion": 3,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "decisions": { "points": [] },
          "actionItems": { "points": [] },
          "openQuestions": { "points": [] },
          "entities": [
            { "name": "AUTH-123", "kind": "ticket", "segmentRefs": [{ "direction": "inbound", "sequence": 7 }] },
            { "name": " Redis ", "kind": "SYSTEM", "segmentRefs": [] },
            { "name": "AUTH-123", "kind": "ticket", "segmentRefs": [] },
            { "name": "  ", "kind": "ticket", "segmentRefs": [] },
            { "name": "Foo", "kind": "weird", "segmentRefs": [] }
          ]
        }"#;
        let parsed = parse_meeting_brief(json).unwrap();
        assert_eq!(parsed.entities.len(), 3);
        assert_eq!(parsed.entities[0].name, "AUTH-123");
        assert_eq!(parsed.entities[0].kind, "ticket");
        assert_eq!(parsed.entities[0].segment_refs[0].sequence, 7);
        assert_eq!(parsed.entities[1].name, "Redis");
        assert_eq!(parsed.entities[1].kind, "system");
        assert_eq!(parsed.entities[2].kind, "generic");
    }

    #[test]
    fn parses_optional_decision_supersedes_index() {
        let json = r#"{
          "schemaVersion": 4,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "decisions": {
            "points": [
              { "id": "de-1", "text": "Use Redis", "segmentRefs": [] },
              { "id": "de-2", "text": "Use PostgreSQL", "supersedesIndex": 1, "segmentRefs": [] }
            ]
          },
          "actionItems": { "points": [] },
          "openQuestions": { "points": [] }
        }"#;
        let parsed = parse_meeting_brief(json).unwrap();
        assert_eq!(parsed.decisions.points[1].supersedes_index, Some(1));
        assert_eq!(parsed.schema_version, MEETING_BRIEF_SCHEMA_VERSION);
    }

    #[test]
    fn rejects_non_array_entities() {
        let json = r#"{
          "schemaVersion": 3,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "entities": "nope"
        }"#;
        assert!(parse_meeting_brief(json).is_err());
    }

    #[test]
    fn parses_standup_with_new_blocks() {
        let json = r#"{
          "schemaVersion": 4,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "decisions": { "points": [] },
          "actionItems": { "points": [] },
          "openQuestions": { "points": [] },
          "yesterday": {
            "points": [
              { "id": "ye-1", "text": "Shipped v2", "segmentRefs": [{ "direction": "outbound", "sequence": 1 }] }
            ]
          },
          "today": {
            "points": [
              { "id": "to-1", "text": "Start v3 planning", "segmentRefs": [{ "direction": "outbound", "sequence": 3 }] }
            ]
          },
          "blockers": {
            "points": [
              { "id": "bl-1", "text": "Waiting on API keys", "segmentRefs": [{ "direction": "inbound", "sequence": 5 }] }
            ]
          }
        }"#;
        let parsed = parse_meeting_brief(json).unwrap();
        assert_eq!(parsed.schema_version, MEETING_BRIEF_SCHEMA_VERSION);
        assert_eq!(parsed.yesterday.points.len(), 1);
        assert_eq!(parsed.yesterday.points[0].id, "ye-1");
        assert_eq!(parsed.yesterday.points[0].text, "Shipped v2");
        assert_eq!(parsed.today.points.len(), 1);
        assert_eq!(parsed.today.points[0].text, "Start v3 planning");
        assert_eq!(parsed.blockers.points.len(), 1);
        assert_eq!(parsed.blockers.points[0].text, "Waiting on API keys");
        assert_eq!(
            block_points(&parsed, SummaryBlockKind::Yesterday)[0].segment_refs[0].sequence,
            1
        );
        assert_eq!(
            block_points(&parsed, SummaryBlockKind::Today)[0].segment_refs[0].sequence,
            3
        );
        assert_eq!(
            block_points(&parsed, SummaryBlockKind::Blockers)[0].segment_refs[0].direction,
            "inbound"
        );
    }

    #[test]
    fn standup_blocks_empty_when_not_in_json() {
        let json = r#"{
          "schemaVersion": 4,
          "overview": { "points": [] },
          "keyPoints": { "points": [] },
          "decisions": { "points": [] },
          "actionItems": { "points": [] },
          "openQuestions": { "points": [] }
        }"#;
        let parsed = parse_meeting_brief(json).unwrap();
        assert!(parsed.yesterday.points.is_empty());
        assert!(parsed.today.points.is_empty());
        assert!(parsed.blockers.points.is_empty());
    }
}
