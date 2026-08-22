//! Full-text search over transcript segments (FTS5).
//!
//! Lexical retrieve seam for Library and timeline search.
//! Indexes both `source_text` and `translated_text` with
//! `unicode61 remove_diacritics 2`.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::meeting::models::{MeetingSearchHit, SegmentSearchHit, TranscriptSegmentView};

use super::MeetingStore;

const DEFAULT_SEARCH_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 100;

/// Strip FTS5 operators; quote each token so reserved words (`AND`/`OR`/`NOT`)
/// are matched as literals rather than boolean operators.
pub fn sanitize_fts5_query(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut cleaned = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            '"' | '*' | '(' | ')' | ':' | '^' | '{' | '}' | '[' | ']' => cleaned.push(' '),
            // Strip common sentence punctuation so "budget?" still matches budget.
            '?' | '!' | ',' | ';' | '.' | '…' | '¿' | '¡' => cleaned.push(' '),
            c if c.is_whitespace() => cleaned.push(' '),
            c => cleaned.push(c),
        }
    }

    let tokens: Vec<String> = cleaned
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t.replace('"', "")))
        .collect();
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(" "))
}

fn clamp_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT)
}

pub(crate) fn index_segment_fts(conn: &Connection, view: &TranscriptSegmentView) -> Result<()> {
    conn.execute(
        "INSERT INTO transcript_fts(segment_id, meeting_id, source_text, translated_text)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            view.id,
            view.meeting_id,
            view.source_text,
            view.translated_text
        ],
    )
    .context("index segment into transcript_fts")?;
    Ok(())
}

impl MeetingStore {
    pub fn backfill_transcript_fts(&self) -> Result<usize> {
        let conn = self.conn();
        let inserted = conn
            .execute(
                "INSERT INTO transcript_fts(segment_id, meeting_id, source_text, translated_text)
                 SELECT id, meeting_id, source_text, translated_text
                 FROM transcript_segment s
                 WHERE NOT EXISTS (
                   SELECT 1 FROM transcript_fts f WHERE f.segment_id = s.id
                 )",
                [],
            )
            .context("backfill transcript_fts")?;
        Ok(inserted)
    }

    pub fn search_segments(
        &self,
        query: &str,
        meeting_id: Option<&str>,
        folder_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<SegmentSearchHit>> {
        let Some(match_query) = sanitize_fts5_query(query) else {
            return Ok(Vec::new());
        };
        let limit = clamp_limit(limit) as i64;
        let conn = self.conn();

        let mut stmt = conn
            .prepare(
                "SELECT
                    f.meeting_id,
                    f.segment_id,
                    snippet(transcript_fts, 2, '', '', '…', 32) AS snip_source,
                    snippet(transcript_fts, 3, '', '', '…', 32) AS snip_translated,
                    bm25(transcript_fts) AS rank,
                    s.started_at_ms,
                    s.direction,
                    s.source_text,
                    s.translated_text
                 FROM transcript_fts f
                 JOIN transcript_segment s ON s.id = f.segment_id
                 JOIN meeting_record m ON m.id = f.meeting_id
                 WHERE transcript_fts MATCH ?1
                   AND (?2 IS NULL OR f.meeting_id = ?2)
                   AND (?3 IS NULL OR m.folder_id = ?3)
                 ORDER BY rank
                 LIMIT ?4",
            )
            .context("prepare search_segments")?;

        let rows = stmt
            .query_map(params![match_query, meeting_id, folder_id, limit], |row| {
                let snip_source: String = row.get(2)?;
                let snip_translated: String = row.get(3)?;
                let source_text: String = row.get(7)?;
                let translated_text: String = row.get(8)?;
                let snippet = pick_snippet(
                    &snip_source,
                    &snip_translated,
                    &source_text,
                    &translated_text,
                );
                Ok(SegmentSearchHit {
                    meeting_id: row.get(0)?,
                    segment_id: row.get(1)?,
                    snippet,
                    rank: row.get(4)?,
                    started_at_ms: Some(row.get(5)?),
                    direction: Some(row.get(6)?),
                })
            })
            .context("query search_segments")?;

        let mut hits = Vec::new();
        for row in rows {
            hits.push(row.context("map search_segments row")?);
        }
        Ok(hits)
    }

    pub fn search_meetings(
        &self,
        query: &str,
        folder_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<MeetingSearchHit>> {
        let Some(match_query) = sanitize_fts5_query(query) else {
            return Ok(Vec::new());
        };
        let limit = clamp_limit(limit) as i64;
        let conn = self.conn();

        // Fetch more segment hits then collapse to best per meeting.
        let fetch_limit = (limit * 8).min(500);
        let mut stmt = conn
            .prepare(
                "SELECT
                    f.meeting_id,
                    f.segment_id,
                    snippet(transcript_fts, 2, '', '', '…', 32) AS snip_source,
                    snippet(transcript_fts, 3, '', '', '…', 32) AS snip_translated,
                    bm25(transcript_fts) AS rank,
                    m.title,
                    s.started_at_ms,
                    s.source_text,
                    s.translated_text
                 FROM transcript_fts f
                 JOIN transcript_segment s ON s.id = f.segment_id
                 JOIN meeting_record m ON m.id = f.meeting_id
                 WHERE transcript_fts MATCH ?1
                   AND (?2 IS NULL OR m.folder_id = ?2)
                 ORDER BY rank
                 LIMIT ?3",
            )
            .context("prepare search_meetings")?;

        let rows = stmt
            .query_map(params![match_query, folder_id, fetch_limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })
            .context("query search_meetings")?;

        let mut best: Vec<MeetingSearchHit> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in rows {
            let (
                meeting_id,
                segment_id,
                snip_source,
                snip_translated,
                rank,
                title,
                segment_started_at_ms,
                source_text,
                translated_text,
            ) = row.context("map search_meetings row")?;
            if !seen.insert(meeting_id.clone()) {
                continue;
            }
            let snippet = pick_snippet(
                &snip_source,
                &snip_translated,
                &source_text,
                &translated_text,
            );
            best.push(MeetingSearchHit {
                meeting_id,
                title,
                snippet,
                rank,
                best_segment_id: segment_id,
                started_at_ms: Some(segment_started_at_ms),
            });
            if best.len() as i64 >= limit {
                break;
            }
        }
        Ok(best)
    }

    /// Test helper: count FTS rows for a segment (used by unit tests).
    #[cfg(test)]
    pub fn fts_row_count_for_segment(&self, segment_id: &str) -> Result<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM transcript_fts WHERE segment_id = ?1",
            params![segment_id],
            |r| r.get(0),
        )
        .map_err(Into::into)
    }

    /// Test helper: insert into transcript_segment without FTS (simulate pre-migration rows).
    #[cfg(test)]
    pub fn insert_segment_without_fts(
        &self,
        meeting_id: &str,
        direction: &str,
        source_text: &str,
        translated_text: &str,
    ) -> Result<String> {
        use uuid::Uuid;
        let conn = self.conn();
        let sequence: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM transcript_segment
             WHERE meeting_id = ?1 AND direction = ?2",
            params![meeting_id, direction],
            |r| r.get(0),
        )?;
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO transcript_segment
             (id, meeting_id, direction, sequence, source_text, translated_text, started_at_ms, ended_at_ms, connection_gap)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 1, 0)",
            params![id, meeting_id, direction, sequence, source_text, translated_text],
        )?;
        Ok(id)
    }
}

fn pick_snippet(
    snip_source: &str,
    snip_translated: &str,
    source_text: &str,
    translated_text: &str,
) -> String {
    let source = snip_source.trim();
    let translated = snip_translated.trim();
    if !source.is_empty() {
        return source.to_string();
    }
    if !translated.is_empty() {
        return translated.to_string();
    }
    let fallback = if !source_text.trim().is_empty() {
        source_text.trim()
    } else {
        translated_text.trim()
    };
    truncate_chars(fallback, 120).to_string()
}

fn truncate_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod sanitize_tests {
    use super::sanitize_fts5_query;

    #[test]
    fn sanitize_keeps_vietnamese_letters() {
        let q = sanitize_fts5_query("  quyết định  ").unwrap();
        assert_eq!(q, "\"quyết\" \"định\"");
    }

    #[test]
    fn sanitize_strips_operators() {
        let q = sanitize_fts5_query(r#""budget*" (plan) :x^"#).unwrap();
        assert!(!q.contains('*'));
        assert!(q.contains("\"budget\""));
        assert!(q.contains("\"plan\""));
    }

    #[test]
    fn sanitize_empty_and_punctuation() {
        assert!(sanitize_fts5_query("   ").is_none());
        assert!(sanitize_fts5_query("***((()))").is_none());
    }

    #[test]
    fn sanitize_preserves_vi_special_letters() {
        let q = sanitize_fts5_query("ăâêôơưđ").unwrap();
        assert_eq!(q, "\"ăâêôơưđ\"");
    }

    #[test]
    fn sanitize_strips_sentence_punctuation() {
        let q = sanitize_fts5_query("budget?").unwrap();
        assert_eq!(q, "\"budget\"");
        let q = sanitize_fts5_query("plan!").unwrap();
        assert_eq!(q, "\"plan\"");
    }

    #[test]
    fn sanitize_quotes_reserved_words() {
        assert_eq!(sanitize_fts5_query("and").as_deref(), Some("\"and\""));
        assert_eq!(sanitize_fts5_query("OR").as_deref(), Some("\"OR\""));
        assert_eq!(sanitize_fts5_query("not").as_deref(), Some("\"not\""));
        let q = sanitize_fts5_query("plan and budget").unwrap();
        assert_eq!(q, "\"plan\" \"and\" \"budget\"");
        // No bare boolean operators between tokens.
        assert!(!q
            .split_whitespace()
            .any(|t| matches!(t.to_ascii_lowercase().as_str(), "and" | "or" | "not")));
    }
}
