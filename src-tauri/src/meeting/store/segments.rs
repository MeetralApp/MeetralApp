use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::meeting::models::{SegmentListResponse, SegmentNeighborsView, TranscriptSegmentView};

use super::MeetingStore;

impl MeetingStore {
    pub fn insert_segment(
        &self,
        meeting_id: &str,
        direction: &str,
        source_text: &str,
        translated_text: &str,
        started_at_ms: i64,
        ended_at_ms: i64,
        connection_gap: bool,
    ) -> Result<TranscriptSegmentView> {
        let sequence: i32 = {
            let conn = self.conn();
            conn.query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM transcript_segment
             WHERE meeting_id = ?1 AND direction = ?2",
                params![meeting_id, direction],
                |r| r.get(0),
            )?
        };
        self.insert_segment_at_sequence(
            meeting_id,
            direction,
            sequence,
            source_text,
            translated_text,
            started_at_ms,
            ended_at_ms,
            connection_gap,
        )
    }

    pub fn max_segment_sequence(&self, meeting_id: &str, direction: &str) -> Result<i32> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM transcript_segment
             WHERE meeting_id = ?1 AND direction = ?2",
            params![meeting_id, direction],
            |r| r.get(0),
        )
        .map_err(Into::into)
    }

    pub fn insert_segment_at_sequence(
        &self,
        meeting_id: &str,
        direction: &str,
        sequence: i32,
        source_text: &str,
        translated_text: &str,
        started_at_ms: i64,
        ended_at_ms: i64,
        connection_gap: bool,
    ) -> Result<TranscriptSegmentView> {
        let mut conn = self.conn();
        let id = Uuid::new_v4().to_string();
        let tx = conn.transaction().context("begin segment insert tx")?;
        match tx.execute(
            "INSERT INTO transcript_segment
             (id, meeting_id, direction, sequence, source_text, translated_text, started_at_ms, ended_at_ms, connection_gap)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                meeting_id,
                direction,
                sequence,
                source_text,
                translated_text,
                started_at_ms,
                ended_at_ms,
                connection_gap as i32
            ],
        ) {
            Ok(_) => {
                let view = TranscriptSegmentView {
                    id,
                    meeting_id: meeting_id.to_string(),
                    direction: direction.to_string(),
                    sequence,
                    source_text: source_text.to_string(),
                    translated_text: translated_text.to_string(),
                    started_at_ms,
                    ended_at_ms,
                    connection_gap,
                };
                super::fts::index_segment_fts(&tx, &view)?;
                tx.commit().context("commit segment insert tx")?;
                Ok(view)
            }
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
 // Existing row — text unchanged; do not re-index FTS.
                let existing = tx.query_row(
                    "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                            started_at_ms, ended_at_ms, connection_gap
                     FROM transcript_segment
                     WHERE meeting_id = ?1 AND direction = ?2 AND sequence = ?3",
                    params![meeting_id, direction, sequence],
                    map_segment,
                )?;
                tx.commit().context("commit segment conflict tx")?;
                Ok(existing)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_segments(
        &self,
        meeting_id: &str,
        direction: Option<&str>,
        from_sequence: Option<i32>,
        before_sequence: Option<i32>,
        tail: bool,
        limit: i64,
    ) -> Result<SegmentListResponse> {
        let conn = self.conn();

        if tail {
            let dir = direction.context("tail mode requires direction")?;
            let total: i64 = conn.query_row(
                "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1 AND direction = ?2",
                params![meeting_id, dir],
                |r| r.get(0),
            )?;
            let mut stmt = conn.prepare(
                "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                        started_at_ms, ended_at_ms, connection_gap
                 FROM transcript_segment
                 WHERE meeting_id = ?1 AND direction = ?2
                 ORDER BY sequence DESC LIMIT ?3",
            )?;
            let mut segs: Vec<TranscriptSegmentView> = stmt
                .query_map(params![meeting_id, dir, limit], map_segment)?
                .collect::<Result<Vec<_>, _>>()?;
            segs.reverse();
            let has_more_older = compute_has_more_older(&conn, meeting_id, dir, &segs)?;
            return Ok(SegmentListResponse {
                segments: segs,
                total,
                has_more_older,
            });
        }

        if let (Some(dir), Some(before)) = (direction, before_sequence) {
            let total: i64 = conn.query_row(
                "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1 AND direction = ?2",
                params![meeting_id, dir],
                |r| r.get(0),
            )?;
            let mut stmt = conn.prepare(
                "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                        started_at_ms, ended_at_ms, connection_gap
                 FROM transcript_segment
                 WHERE meeting_id = ?1 AND direction = ?2 AND sequence < ?3
                 ORDER BY sequence DESC LIMIT ?4",
            )?;
            let mut segs: Vec<TranscriptSegmentView> = stmt
                .query_map(params![meeting_id, dir, before, limit], map_segment)?
                .collect::<Result<Vec<_>, _>>()?;
            segs.reverse();
            let has_more_older = compute_has_more_older(&conn, meeting_id, dir, &segs)?;
            return Ok(SegmentListResponse {
                segments: segs,
                total,
                has_more_older,
            });
        }

        let (total, segments) = match (direction, from_sequence) {
            (Some(dir), Some(from)) => {
                let total: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1 AND direction = ?2 AND sequence >= ?3",
                    params![meeting_id, dir, from],
                    |r| r.get(0),
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                            started_at_ms, ended_at_ms, connection_gap
                     FROM transcript_segment
                     WHERE meeting_id = ?1 AND direction = ?2 AND sequence >= ?3
                     ORDER BY sequence ASC LIMIT ?4",
                )?;
                let segs = stmt
                    .query_map(params![meeting_id, dir, from, limit], map_segment)?
                    .collect::<Result<Vec<_>, _>>()?;
                (total, segs)
            }
            (Some(dir), None) => {
                let total: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1 AND direction = ?2",
                    params![meeting_id, dir],
                    |r| r.get(0),
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                            started_at_ms, ended_at_ms, connection_gap
                     FROM transcript_segment
                     WHERE meeting_id = ?1 AND direction = ?2
                     ORDER BY sequence ASC LIMIT ?3",
                )?;
                let segs = stmt
                    .query_map(params![meeting_id, dir, limit], map_segment)?
                    .collect::<Result<Vec<_>, _>>()?;
                (total, segs)
            }
            (None, _) => {
                let total: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1",
                    params![meeting_id],
                    |r| r.get(0),
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                            started_at_ms, ended_at_ms, connection_gap
                     FROM transcript_segment
                     WHERE meeting_id = ?1
                     ORDER BY direction ASC, sequence ASC LIMIT ?2",
                )?;
                let segs = stmt
                    .query_map(params![meeting_id, limit], map_segment)?
                    .collect::<Result<Vec<_>, _>>()?;
                (total, segs)
            }
        };
        Ok(SegmentListResponse {
            segments,
            total,
            has_more_older: false,
        })
    }

    pub fn list_segments_chronological_page(
        &self,
        meeting_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<TranscriptSegmentView>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                    started_at_ms, ended_at_ms, connection_gap
             FROM transcript_segment
             WHERE meeting_id = ?1
             ORDER BY started_at_ms ASC, sequence ASC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt
            .query_map(params![meeting_id, limit, offset], map_segment)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_segments(&self, meeting_id: &str) -> Result<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM transcript_segment WHERE meeting_id = ?1",
            params![meeting_id],
            |r| r.get(0),
        )
        .map_err(Into::into)
    }

    pub fn max_segment_started_at_ms(&self, meeting_id: &str) -> Result<Option<i64>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT MAX(started_at_ms) FROM transcript_segment WHERE meeting_id = ?1",
            params![meeting_id],
            |r| r.get(0),
        )
        .map_err(Into::into)
    }

    /// Last `limit` segments for a meeting, returned in chronological order.
    /// Empty when the meeting has no segments yet.
    pub fn list_last_segments(
        &self,
        meeting_id: &str,
        limit: i64,
    ) -> Result<Vec<TranscriptSegmentView>> {
        let limit = limit.max(0);
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                    started_at_ms, ended_at_ms, connection_gap
             FROM transcript_segment
             WHERE meeting_id = ?1
             ORDER BY started_at_ms DESC
             LIMIT ?2",
        )?;
        let mut segs = stmt
            .query_map(params![meeting_id, limit], map_segment)?
            .collect::<Result<Vec<_>, _>>()?;
        segs.reverse();
        Ok(segs)
    }

    /// Exact rows for a set of segment ids in one meeting (citation metadata).
    pub fn list_segments_by_ids(
        &self,
        meeting_id: &str,
        ids: &[String],
    ) -> Result<Vec<TranscriptSegmentView>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                    started_at_ms, ended_at_ms, connection_gap
             FROM transcript_segment
             WHERE meeting_id = ? AND id IN ({placeholders})"
        );
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(ids.len() + 1);
        params.push(Box::new(meeting_id.to_string()));
        for id in ids {
            params.push(Box::new(id.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), map_segment)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_segment(&self, segment_id: &str) -> Result<TranscriptSegmentView> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                    started_at_ms, ended_at_ms, connection_gap
             FROM transcript_segment WHERE id = ?1",
            params![segment_id],
            map_segment,
        )
        .context("segment not found")
    }

    pub fn get_segment_neighbors(
        &self,
        meeting_id: &str,
        segment_id: &str,
    ) -> Result<SegmentNeighborsView> {
        let current = self.get_segment(segment_id)?;
        if current.meeting_id != meeting_id {
            anyhow::bail!("segment does not belong to meeting");
        }
        let conn = self.conn();
        let prev: Option<TranscriptSegmentView> = conn
            .query_row(
                "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                        started_at_ms, ended_at_ms, connection_gap
                 FROM transcript_segment
                 WHERE meeting_id = ?1 AND direction = ?2 AND sequence < ?3
                 ORDER BY sequence DESC LIMIT 1",
                params![meeting_id, current.direction, current.sequence],
                map_segment,
            )
            .optional()?;
        let next: Option<TranscriptSegmentView> = conn
            .query_row(
                "SELECT id, meeting_id, direction, sequence, source_text, translated_text,
                        started_at_ms, ended_at_ms, connection_gap
                 FROM transcript_segment
                 WHERE meeting_id = ?1 AND direction = ?2 AND sequence > ?3
                 ORDER BY sequence ASC LIMIT 1",
                params![meeting_id, current.direction, current.sequence],
                map_segment,
            )
            .optional()?;
        Ok(SegmentNeighborsView {
            prev,
            current,
            next,
        })
    }

    pub fn segment_id_by_sequence(
        &self,
        meeting_id: &str,
        direction: &str,
        sequence: i32,
    ) -> Result<Option<String>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id FROM transcript_segment
             WHERE meeting_id = ?1 AND direction = ?2 AND sequence = ?3",
            params![meeting_id, direction, sequence],
            |r| r.get(0),
        )
        .optional()
        .context("lookup segment")
    }
}

fn compute_has_more_older(
    conn: &rusqlite::Connection,
    meeting_id: &str,
    direction: &str,
    segments: &[TranscriptSegmentView],
) -> Result<bool> {
    let Some(min_seq) = segments.iter().map(|s| s.sequence).min() else {
        return Ok(false);
    };
    let older: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transcript_segment
         WHERE meeting_id = ?1 AND direction = ?2 AND sequence < ?3",
        params![meeting_id, direction, min_seq],
        |r| r.get(0),
    )?;
    Ok(older > 0)
}

fn map_segment(row: &rusqlite::Row<'_>) -> rusqlite::Result<TranscriptSegmentView> {
    Ok(TranscriptSegmentView {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        direction: row.get(2)?,
        sequence: row.get(3)?,
        source_text: row.get(4)?,
        translated_text: row.get(5)?,
        started_at_ms: row.get(6)?,
        ended_at_ms: row.get(7)?,
        connection_gap: row.get::<_, i32>(8)? != 0,
    })
}
