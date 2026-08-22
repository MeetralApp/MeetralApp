use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::meeting::models::{MeetingListResponse, MeetingRecordView, MeetingStatus};

use super::paths::wall_ms;
use super::MeetingStore;

impl MeetingStore {
    pub fn create_meeting(
        &self,
        title: &str,
        folder_id: Option<&str>,
        my_language: &str,
        meeting_language: &str,
        session_mode: &str,
        status: MeetingStatus,
    ) -> Result<MeetingRecordView> {
        let id = Uuid::new_v4().to_string();
        let now = wall_ms();
        let mode = if session_mode == "notes" {
            "notes"
        } else {
            "interpreter"
        };
        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO meeting_record
                 (id, folder_id, title, my_language, meeting_language, session_mode, started_at_ms, ended_at_ms, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
                params![
                    id,
                    folder_id,
                    title,
                    my_language,
                    meeting_language,
                    mode,
                    now,
                    status.as_str()
                ],
            )?;
        }
        self.get_meeting(&id, true)
    }

    pub fn end_meeting(&self, id: &str) -> Result<MeetingRecordView> {
        let now = wall_ms();
        {
            let conn = self.conn();
            let updated = conn.execute(
                "UPDATE meeting_record SET status = 'ended', ended_at_ms = ?1 WHERE id = ?2",
                params![now, id],
            )?;
            if updated == 0 {
                anyhow::bail!("meeting not found");
            }
        }
        self.get_meeting(id, true)
    }

    pub fn rename_meeting(&self, id: &str, title: &str) -> Result<MeetingRecordView> {
        {
            let conn = self.conn();
            let updated = conn.execute(
                "UPDATE meeting_record SET title = ?1 WHERE id = ?2",
                params![title, id],
            )?;
            if updated == 0 {
                anyhow::bail!("meeting not found");
            }
        }
        self.get_meeting(id, true)
    }

    pub fn move_meeting(&self, id: &str, folder_id: Option<&str>) -> Result<MeetingRecordView> {
        {
            let conn = self.conn();
            let updated = conn.execute(
                "UPDATE meeting_record SET folder_id = ?1 WHERE id = ?2",
                params![folder_id, id],
            )?;
            if updated == 0 {
                anyhow::bail!("meeting not found");
            }
        }
        self.get_meeting(id, true)
    }

    pub fn delete_meeting(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        let deleted = conn.execute("DELETE FROM meeting_record WHERE id = ?1", params![id])?;
        if deleted == 0 {
            anyhow::bail!("meeting not found");
        }
        Ok(())
    }

    pub fn list_meetings(
        &self,
        folder_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<MeetingListResponse> {
        let (total, ids) = {
            let conn = self.conn();
            match folder_id {
                Some("__uncategorized__") => {
                    let total: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM meeting_record WHERE folder_id IS NULL",
                        [],
                        |r| r.get(0),
                    )?;
                    let mut stmt = conn.prepare(
                        "SELECT id FROM meeting_record WHERE folder_id IS NULL
                     ORDER BY started_at_ms DESC LIMIT ?1 OFFSET ?2",
                    )?;
                    let ids: Vec<String> = stmt
                        .query_map(params![limit, offset], |r| r.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    (total, ids)
                }
                Some(fid) => {
                    let total: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM meeting_record WHERE folder_id = ?1",
                        params![fid],
                        |r| r.get(0),
                    )?;
                    let mut stmt = conn.prepare(
                        "SELECT id FROM meeting_record WHERE folder_id = ?1
                     ORDER BY started_at_ms DESC LIMIT ?2 OFFSET ?3",
                    )?;
                    let ids: Vec<String> = stmt
                        .query_map(params![fid, limit, offset], |r| r.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    (total, ids)
                }
                None => {
                    let total: i64 =
                        conn.query_row("SELECT COUNT(*) FROM meeting_record", [], |r| r.get(0))?;
                    let mut stmt = conn.prepare(
                    "SELECT id FROM meeting_record ORDER BY started_at_ms DESC LIMIT ?1 OFFSET ?2",
                )?;
                    let ids: Vec<String> = stmt
                        .query_map(params![limit, offset], |r| r.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    (total, ids)
                }
            }
        };
        let meetings = ids
            .into_iter()
            .map(|id| self.get_meeting(&id, true))
            .collect::<Result<Vec<_>>>()?;
        Ok(MeetingListResponse { meetings, total })
    }

    pub fn get_meeting(&self, id: &str, with_counts: bool) -> Result<MeetingRecordView> {
        let conn = self.conn();
        let base = conn.query_row(
            "SELECT id, folder_id, title, my_language, meeting_language, session_mode, started_at_ms, ended_at_ms, status
             FROM meeting_record WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get(8)?;
                Ok(MeetingRecordView {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    title: row.get(2)?,
                    my_language: row.get(3)?,
                    meeting_language: row.get(4)?,
                    session_mode: row.get(5)?,
                    started_at_ms: row.get(6)?,
                    ended_at_ms: row.get(7)?,
                    status: status_str.parse().unwrap_or(MeetingStatus::Ended),
                    segment_count_outbound: None,
                    segment_count_inbound: None,
                    has_summary: None,
                })
            },
        )?;

        if !with_counts {
            return Ok(base);
        }

        let outbound: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1 AND direction = 'outbound'",
            params![id],
            |r| r.get(0),
        )?;
        let inbound: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1 AND direction = 'inbound'",
            params![id],
            |r| r.get(0),
        )?;
        let has_summary: i64 = conn.query_row(
            "SELECT COUNT(*) FROM meeting_summary
             WHERE meeting_id = ?1 AND generated_json != '' AND generated_json != '{}'",
            params![id],
            |r| r.get(0),
        )?;

        Ok(MeetingRecordView {
            segment_count_outbound: Some(outbound),
            segment_count_inbound: Some(inbound),
            has_summary: Some(has_summary > 0),
            ..base
        })
    }

    pub fn get_live_meeting(&self) -> Result<Option<MeetingRecordView>> {
        let id: Option<String> = {
            let conn = self.conn();
            conn.query_row(
                "SELECT id FROM meeting_record WHERE status = 'live' ORDER BY started_at_ms DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?
        };
        match id {
            Some(id) => Ok(Some(self.get_meeting(&id, true)?)),
            None => Ok(None),
        }
    }
}
