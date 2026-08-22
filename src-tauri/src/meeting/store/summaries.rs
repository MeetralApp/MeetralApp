use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::meeting::models::MeetingSummaryView;
use crate::meeting::summary_doc::doc_to_prose;

use super::paths::wall_ms;
use super::MeetingStore;

impl MeetingStore {
    pub fn get_summary(&self, meeting_id: &str) -> Result<Option<MeetingSummaryView>> {
        let row: Option<(String, String, String, Option<i64>)> = {
            let conn = self.conn();
            conn.query_row(
                "SELECT template_id, summary_language, generated_json, generated_at_ms
                 FROM meeting_summary WHERE meeting_id = ?1",
                params![meeting_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
        };
        let Some((template_id, summary_language, generated_json, generated_at_ms)) = row else {
            return Ok(None);
        };
        let generated_text = match serde_json::from_str::<Value>(&generated_json) {
            Ok(doc) => doc_to_prose(&doc),
            Err(_) => String::new(),
        };
        Ok(Some(MeetingSummaryView {
            meeting_id: meeting_id.to_string(),
            template_id,
            summary_language,
            generated_json,
            generated_text,
            generated_at_ms,
        }))
    }

    /// Persist TipTap SSOT (generate or user edit overwrite).
    pub fn save_summary(
        &self,
        meeting_id: &str,
        template_id: &str,
        summary_language: &str,
        generated_json: &str,
    ) -> Result<MeetingSummaryView> {
        let _: Value = serde_json::from_str(generated_json).context("invalid TipTap doc JSON")?;
        let now = wall_ms();
        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO meeting_summary (meeting_id, template_id, summary_language, generated_json, generated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(meeting_id) DO UPDATE SET
                   template_id = excluded.template_id,
                   summary_language = excluded.summary_language,
                   generated_json = excluded.generated_json,
                   generated_at_ms = excluded.generated_at_ms",
                params![
                    meeting_id,
                    template_id,
                    summary_language,
                    generated_json,
                    now
                ],
            )?;
        }
        self.get_summary(meeting_id)?
            .ok_or_else(|| anyhow::anyhow!("summary missing"))
    }

    /// Overwrite TipTap SSOT (user edit). Same storage as generate.
    pub fn update_summary_doc(
        &self,
        meeting_id: &str,
        doc_json: &str,
    ) -> Result<MeetingSummaryView> {
        let trimmed = doc_json.trim();
        if trimmed.is_empty() {
            anyhow::bail!("summary document is empty");
        }
        let existing = self
            .get_summary(meeting_id)?
            .ok_or_else(|| anyhow::anyhow!("summary not found — generate first"))?;
        self.save_summary(
            meeting_id,
            &existing.template_id,
            &existing.summary_language,
            trimmed,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::meeting::models::MeetingStatus;
    use crate::meeting::store::MeetingStore;

    #[test]
    fn save_and_update_summary_doc_persist_tiptap() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        let seg = store
            .insert_segment(&m.id, "outbound", "hello", "xin chao", 0, 100, false)
            .unwrap();

        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Note " },
                    { "type": "citation", "attrs": {
                        "segmentId": seg.id,
                        "direction": "outbound",
                        "startedAtMs": 0
                    }}
                ]
            }]
        });
        let view = store
            .save_summary(&m.id, "meeting_brief", "en", &doc.to_string())
            .unwrap();
        assert!(view.generated_json.contains("citation"));
        assert_eq!(view.generated_text, "Note 0:00·You");

        let updated_doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Updated " },
                    { "type": "citation", "attrs": {
                        "segmentId": seg.id,
                        "direction": "outbound",
                        "startedAtMs": 0
                    }}
                ]
            }]
        });
        let updated = store
            .update_summary_doc(&m.id, &updated_doc.to_string())
            .unwrap();
        assert_eq!(updated.generated_text, "Updated 0:00·You");
    }
}
