//! Structured artifacts store (meeting_artifact + meeting_entity + entity_fts).
//! Rows are regenerated from the parsed meeting brief; user-set
//! statuses live on artifact rows and reset on regenerate (replace is clean).

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::{sanitize_fts5_query, MeetingStore};
use crate::meeting::models::SegmentCitation;
use crate::meeting::summary_schema::{MeetingBriefSummary, SegmentRef, SummaryPoint};

pub const ARTIFACT_KIND_DECISION: &str = "decision";
pub const ARTIFACT_KIND_ACTION_ITEM: &str = "action_item";
pub const ARTIFACT_KIND_OPEN_QUESTION: &str = "open_question";
pub const ARTIFACT_STATUSES: [&str; 4] = ["proposed", "confirmed", "dismissed", "superseded"];
pub const ARTIFACT_ORIGIN_EXTRACTED: &str = "extracted";
pub const ARTIFACT_ORIGIN_EDITED: &str = "edited";
pub const ARTIFACT_ORIGIN_MANUAL: &str = "manual";

/// Min length of the shorter side for a containment dedupe match —
/// guards against trivial substring hits on tiny strings ("OK", "deploy").
pub const ARTIFACT_DEDUPE_MIN_CONTAINMENT_CHARS: usize = 10;

/// Lowercase Unicode, non-alphanumeric → space, collapse whitespace
/// (soft-dedupe). Diacritics are kept: re-extraction is usually byte-identical.
fn normalize_match_text(s: &str) -> String {
    let lowered: String = s
        .chars()
        .flat_map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().collect::<Vec<_>>()
            } else {
                vec![' ']
            }
        })
        .collect();
    lowered.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Exact or containment match on normalized text. Containment only
/// counts when the shorter side is ≥ ARTIFACT_DEDUPE_MIN_CONTAINMENT_CHARS.
fn artifact_texts_match(a: &str, b: &str) -> bool {
    let (na, nb) = (normalize_match_text(a), normalize_match_text(b));
    if na.is_empty() || nb.is_empty() {
        return false;
    }
    if na == nb {
        return true;
    }
    let (short, long) = if na.len() <= nb.len() {
        (&na, &nb)
    } else {
        (&nb, &na)
    };
    short.chars().count() >= ARTIFACT_DEDUPE_MIN_CONTAINMENT_CHARS && long.contains(short.as_str())
}

/// Namespace for rows the user created or that were re-keyed after a content
/// edit. These survive summary regenerate (positional extracted ids are wiped
/// and reused, so user content must never occupy that namespace).
pub fn user_artifact_id(meeting_id: &str, kind: &str, salt: &str) -> String {
    format!("{meeting_id}:{kind}:u{salt}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRow {
    pub id: String,
    pub meeting_id: String,
    pub kind: String,
    pub text: String,
    pub owner: Option<String>,
    pub due: Option<String>,
    pub status: String,
    pub supersedes_artifact_id: Option<String>,
    pub segment_refs: Vec<SegmentRef>,
    pub created_at_ms: i64,
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRow {
    pub id: String,
    pub meeting_id: String,
    pub name: String,
    pub kind: String,
    pub segment_refs: Vec<SegmentRef>,
    pub origin: String,
}

impl MeetingStore {
    /// Wipe + rewrite all artifacts/entities for a meeting from a parsed brief,
    /// in one transaction. Returns the artifact row count.
    pub fn replace_meeting_artifacts(
        &self,
        meeting_id: &str,
        summary: &MeetingBriefSummary,
        now_ms: i64,
    ) -> Result<usize> {
        let mut guard = self.conn();
        let tx = guard
            .transaction()
            .context("begin replace_meeting_artifacts tx")?;
        // Only wipe rows the LLM extracted; user-edited / manually-added rows
        // (origin != 'extracted') survive a regenerate.
        tx.execute(
            "DELETE FROM meeting_artifact WHERE meeting_id = ?1 AND origin = 'extracted'",
            params![meeting_id],
        )?;
        tx.execute(
            "DELETE FROM meeting_entity WHERE meeting_id = ?1 AND origin = 'extracted'",
            params![meeting_id],
        )?;

        // Null dangling supersession links on surviving rows BEFORE the new
        // insert: positional extracted ids are about to be reused by the fresh
        // batch, so a surviving link would otherwise point at an unrelated new
        // decision. Deleted extracted targets are gone; surviving targets stay.
        null_dangling_supersedes(&tx, meeting_id)
            .context("null dangling supersedes_artifact_id")?;

        // Snapshot surviving user rows for soft-dedupe: after
        // the extracted wipe, everything left is user content. A fresh
        // extracted point matching a survivor (same kind, normalized text) is
        // skipped — the user's version wins, re-extraction must not duplicate.
        let surviving_artifacts: Vec<(String, String)> = tx
            .prepare("SELECT kind, text FROM meeting_artifact WHERE meeting_id = ?1")?
            .query_map(params![meeting_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let surviving_entity_names: Vec<String> = tx
            .prepare("SELECT name FROM meeting_entity WHERE meeting_id = ?1")?
            .query_map(params![meeting_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Summary extraction emits one-based indexes into its decision array.
        // Re-derive these links on every clean replace; old rows are deleted.
        let superseded_decision_indexes: HashSet<usize> = summary
            .decisions
            .points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| {
                point
                    .supersedes_index
                    .filter(|target| *target > 0 && *target <= index)
            })
            .collect();

        let mut count = 0usize;
        let mut deduped = 0usize;
        let mut insert_points = |kind: &str, points: &[SummaryPoint]| -> Result<()> {
            for (index, point) in points.iter().enumerate() {
                if surviving_artifacts
                    .iter()
                    .any(|(k, text)| k == kind && artifact_texts_match(text, &point.text))
                {
                    deduped += 1;
                    continue;
                }
                let id = format!("{meeting_id}:{kind}:{}", index + 1);
                let refs = serde_json::to_string(&point.segment_refs)?;
                let supersedes_artifact_id = (kind == ARTIFACT_KIND_DECISION)
                    .then_some(point.supersedes_index)
                    .flatten()
                    .filter(|target| *target > 0 && *target <= index)
                    .map(|target| format!("{meeting_id}:{kind}:{target}"));
                let status = if kind == ARTIFACT_KIND_DECISION
                    && superseded_decision_indexes.contains(&(index + 1))
                {
                    "superseded"
                } else {
                    "proposed"
                };
                tx.execute(
                    "INSERT INTO meeting_artifact \
                     (id, meeting_id, topic_id, kind, text, owner, due, status, supersedes_artifact_id, segment_refs, created_at_ms) \
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        id,
                        meeting_id,
                        kind,
                        point.text,
                        point.owner,
                        point.due,
                        status,
                        supersedes_artifact_id,
                        refs,
                        now_ms
                    ],
                )?;
                count += 1;
            }
            Ok(())
        };
        insert_points(ARTIFACT_KIND_DECISION, &summary.decisions.points)?;
        insert_points(ARTIFACT_KIND_ACTION_ITEM, &summary.action_items.points)?;
        insert_points(ARTIFACT_KIND_OPEN_QUESTION, &summary.open_questions.points)?;

        for entity in &summary.entities {
            // Name-column dedupe: a surviving row with the same normalized
            // name (e.g. after rename Jon -> John) wins over a freshly-extracted
            // "John" with a different id (gap 2).
            if surviving_entity_names
                .iter()
                .any(|name| normalize_match_text(name) == normalize_match_text(&entity.name))
            {
                continue;
            }
            let id = format!("{meeting_id}:entity:{}", entity.name);
            let refs = serde_json::to_string(&entity.segment_refs)?;
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO meeting_entity \
                 (id, meeting_id, topic_id, name, kind, segment_refs, created_at_ms, origin) \
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, 'extracted')",
                params![id, meeting_id, entity.name, entity.kind, refs, now_ms],
            )?;
            if inserted == 1 {
                tx.execute(
                    "INSERT INTO entity_fts (entity_id, meeting_id, name) VALUES (?1, ?2, ?3)",
                    params![id, meeting_id, entity.name],
                )?;
            }
        }

        // A fresh decision whose supersession target was dedupe-skipped would
        // otherwise dangle; null those too (same rule as the pre-insert pass).
        null_dangling_supersedes(&tx, meeting_id)
            .context("null dangling supersedes_artifact_id post-insert")?;

        if deduped > 0 {
            tracing::debug!(meeting_id = %meeting_id, deduped, "artifact regenerate skipped duplicate user rows");
        }

        tx.commit().context("commit replace_meeting_artifacts tx")?;
        Ok(count)
    }

    pub fn list_artifacts(
        &self,
        meeting_id: &str,
        kind: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<ArtifactRow>> {
        let conn = self.conn();
        let mut sql = String::from(
            "SELECT id, meeting_id, kind, text, owner, due, status, supersedes_artifact_id, segment_refs, created_at_ms, origin \
             FROM meeting_artifact WHERE meeting_id = ?1",
        );
        if kind.is_some() {
            sql.push_str(" AND kind = ?2");
        }
        if status.is_some() {
            sql.push_str(if kind.is_some() {
                " AND status = ?3"
            } else {
                " AND status = ?2"
            });
        }
        sql.push_str(" ORDER BY rowid");
        let mut stmt = conn.prepare(&sql)?;
        let rows = match (kind, status) {
            (Some(kind), Some(status)) => {
                stmt.query_map(params![meeting_id, kind, status], map_artifact)?
            }
            (Some(kind), None) => stmt.query_map(params![meeting_id, kind], map_artifact)?,
            (None, Some(status)) => stmt.query_map(params![meeting_id, status], map_artifact)?,
            (None, None) => stmt.query_map(params![meeting_id], map_artifact)?,
        };
        let mut artifacts = Vec::new();
        for row in rows {
            artifacts.push(row?);
        }
        Ok(artifacts)
    }

    /// Returns the updated row, or `None` when the id does not exist.
    pub fn set_artifact_status(
        &self,
        artifact_id: &str,
        status: &str,
    ) -> Result<Option<ArtifactRow>> {
        if !ARTIFACT_STATUSES.contains(&status) {
            bail!("invalid artifact status: {status}");
        }
        let conn = self.conn();
        let updated = conn.execute(
            "UPDATE meeting_artifact SET status = ?2 WHERE id = ?1",
            params![artifact_id, status],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        let row = conn.query_row(
            "SELECT id, meeting_id, kind, text, owner, due, status, supersedes_artifact_id, segment_refs, created_at_ms, origin \
             FROM meeting_artifact WHERE id = ?1",
            params![artifact_id],
            map_artifact,
        )?;
        Ok(Some(row))
    }

    /// Edit an artifact's text/owner/due. Re-keys 'extracted' rows into
    /// the user namespace (origin='edited') so a future regenerate never
    /// collides with positional ids. Returns None when the id does not exist;
    /// bails on empty trimmed text.
    pub fn update_artifact_content(
        &self,
        artifact_id: &str,
        text: &str,
        owner: Option<&str>,
        due: Option<&str>,
    ) -> Result<Option<ArtifactRow>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            bail!("artifact text is empty");
        }
        let owner = owner.map(str::trim).filter(|s| !s.is_empty());
        let due = due.map(str::trim).filter(|s| !s.is_empty());

        let existing = self.get_artifact(artifact_id)?;
        let Some(cur) = existing else {
            return Ok(None);
        };

        let lookup_id = {
            let mut guard = self.conn();
            let tx = guard
                .transaction()
                .context("begin update_artifact_content tx")?;
            let lookup_id = if cur.origin == ARTIFACT_ORIGIN_EXTRACTED {
                let salt = &Uuid::new_v4().simple().to_string()[..8];
                let new_id = user_artifact_id(&cur.meeting_id, &cur.kind, salt);
                // FK on supersedes_artifact_id blocks a live PK rename while
                // dependents still point at the old id — clear, rename, restore.
                let dependents: Vec<String> = tx
                    .prepare("SELECT id FROM meeting_artifact WHERE supersedes_artifact_id = ?1")?
                    .query_map(params![artifact_id], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<_>>()?;
                tx.execute(
                    "UPDATE meeting_artifact SET supersedes_artifact_id = NULL \
                     WHERE supersedes_artifact_id = ?1",
                    params![artifact_id],
                )?;
                tx.execute(
                    "UPDATE meeting_artifact \
                     SET id = ?2, text = ?3, owner = ?4, due = ?5, origin = 'edited' \
                     WHERE id = ?1",
                    params![artifact_id, new_id, trimmed, owner, due],
                )?;
                for dep_id in dependents {
                    tx.execute(
                        "UPDATE meeting_artifact SET supersedes_artifact_id = ?2 \
                         WHERE id = ?1",
                        params![dep_id, new_id],
                    )?;
                }
                new_id
            } else {
                tx.execute(
                    "UPDATE meeting_artifact SET text = ?2, owner = ?3, due = ?4 WHERE id = ?1",
                    params![artifact_id, trimmed, owner, due],
                )?;
                artifact_id.to_string()
            };
            tx.commit().context("commit update_artifact_content tx")?;
            lookup_id
        };
        self.get_artifact(&lookup_id)
    }

    /// Create a manually-added artifact: user-namespace id, status
    /// 'proposed', origin 'manual', no segment refs. Bails on invalid kind or
    /// empty text.
    pub fn create_artifact(
        &self,
        meeting_id: &str,
        kind: &str,
        text: &str,
        owner: Option<&str>,
        due: Option<&str>,
        now_ms: i64,
    ) -> Result<ArtifactRow> {
        if !matches!(
            kind,
            ARTIFACT_KIND_DECISION | ARTIFACT_KIND_ACTION_ITEM | ARTIFACT_KIND_OPEN_QUESTION
        ) {
            bail!("invalid artifact kind: {kind}");
        }
        let trimmed = text.trim();
        if trimmed.is_empty() {
            bail!("artifact text is empty");
        }
        let trim_owner = owner.map(str::trim).filter(|s| !s.is_empty());
        let due = due.map(str::trim).filter(|s| !s.is_empty());
        let salt = &Uuid::new_v4().simple().to_string()[..8];
        let id = user_artifact_id(meeting_id, kind, salt);
        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO meeting_artifact \
                 (id, meeting_id, topic_id, kind, text, owner, due, status, supersedes_artifact_id, segment_refs, created_at_ms, origin) \
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, 'proposed', NULL, '[]', ?7, ?8)",
                params![id, meeting_id, kind, trimmed, trim_owner, due, now_ms, ARTIFACT_ORIGIN_MANUAL],
            )?;
        }
        self.get_artifact(&id)
            .map(|row| row.expect("just-inserted artifact row"))
    }

    /// Hard-delete an artifact row. Returns false when id missing.
    /// An 'extracted' row may be re-extracted on the next summary regenerate.
    pub fn delete_artifact(&self, artifact_id: &str) -> Result<bool> {
        let updated = self.conn().execute(
            "DELETE FROM meeting_artifact WHERE id = ?1",
            params![artifact_id],
        )?;
        // Also clear supersession links pointing at the deleted row.
        self.conn().execute(
            "UPDATE meeting_artifact SET supersedes_artifact_id = NULL \
             WHERE supersedes_artifact_id = ?1",
            params![artifact_id],
        )?;
        Ok(updated > 0)
    }

    /// Rename / re-kind an entity. Keeps the name-keyed id stable so a
    /// future regenerate suppresses the old name (INSERT OR IGNORE no-op).
    /// Updates entity_fts in the same transaction. None when id missing.
    /// Errors when another row in the meeting already owns the normalized name
    /// (parity with `create_entity` / regenerate name-column dedupe — gap 2).
    pub fn update_entity_content(
        &self,
        entity_id: &str,
        name: &str,
        kind: &str,
    ) -> Result<Option<EntityRow>> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("entity name is empty");
        }
        let Some(cur) = self.get_entity(entity_id)? else {
            return Ok(None);
        };
        let normalized_trimmed = normalize_match_text(trimmed);
        {
            let mut guard = self.conn();
            let tx = guard
                .transaction()
                .context("begin update_entity_content tx")?;
            // Conflict = another surviving row (different id) with the same
            // normalized name — including case variants (AUTH-123 vs auth-123).
            let conflict = tx
                .prepare(
                    "SELECT id, name FROM meeting_entity \
                     WHERE meeting_id = ?1 AND id != ?2",
                )?
                .query_map(params![cur.meeting_id, entity_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .any(|(_id, n)| normalize_match_text(&n) == normalized_trimmed);
            if conflict {
                bail!("entity already exists: {trimmed}");
            }
            let origin = if cur.origin == ARTIFACT_ORIGIN_EXTRACTED {
                ARTIFACT_ORIGIN_EDITED
            } else {
                cur.origin.as_str()
            };
            tx.execute(
                "UPDATE meeting_entity SET name = ?2, kind = ?3, origin = ?4 WHERE id = ?1",
                params![entity_id, trimmed, kind, origin],
            )?;
            // entity_fts delete trigger only fires on DELETE; maintain FTS manually.
            tx.execute(
                "DELETE FROM entity_fts WHERE entity_id = ?1",
                params![entity_id],
            )?;
            tx.execute(
                "INSERT INTO entity_fts (entity_id, meeting_id, name) VALUES (?1, ?2, ?3)",
                params![entity_id, cur.meeting_id, trimmed],
            )?;
            tx.commit().context("commit update_entity_content tx")?;
        }
        self.get_entity(entity_id)
    }

    /// Create a manually-added entity: name-keyed id, origin 'manual',
    /// empty segment refs, + FTS row. Returns None on name conflict.
    pub fn create_entity(
        &self,
        meeting_id: &str,
        name: &str,
        kind: &str,
        now_ms: i64,
    ) -> Result<Option<EntityRow>> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("entity name is empty");
        }
        let normalized_trimmed = normalize_match_text(trimmed);
        let id = format!("{meeting_id}:entity:{trimmed}");
        let inserted = {
            let mut guard = self.conn();
            let tx = guard.transaction().context("begin create_entity tx")?;
            // Conflict = id match (INSERT OR IGNORE) OR normalized-name match with
            // any existing row, even a renamed one with a different id (gap 2).
            let existing_names: Vec<String> = tx
                .prepare("SELECT name FROM meeting_entity WHERE meeting_id = ?1")?
                .query_map(params![meeting_id], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<_>>()?;
            if existing_names
                .iter()
                .any(|n| normalize_match_text(n) == normalized_trimmed)
            {
                return Ok(None);
            }
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO meeting_entity \
                 (id, meeting_id, topic_id, name, kind, segment_refs, created_at_ms, origin) \
                 VALUES (?1, ?2, NULL, ?3, ?4, '[]', ?5, ?6)",
                params![
                    id,
                    meeting_id,
                    trimmed,
                    kind,
                    now_ms,
                    ARTIFACT_ORIGIN_MANUAL
                ],
            )?;
            if inserted == 0 {
                return Ok(None);
            }
            tx.execute(
                "INSERT INTO entity_fts (entity_id, meeting_id, name) VALUES (?1, ?2, ?3)",
                params![id, meeting_id, trimmed],
            )?;
            tx.commit().context("commit create_entity tx")?;
            inserted
        };
        debug_assert!(inserted > 0);
        self.get_entity(&id)
    }

    /// Hard-delete an entity; the migration-006 delete trigger cleans
    /// entity_fts. False when id missing. 'extracted' rows may re-appear on
    /// the next summary regenerate.
    pub fn delete_entity(&self, entity_id: &str) -> Result<bool> {
        let updated = self.conn().execute(
            "DELETE FROM meeting_entity WHERE id = ?1",
            params![entity_id],
        )?;
        Ok(updated > 0)
    }

    fn get_artifact(&self, artifact_id: &str) -> Result<Option<ArtifactRow>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, meeting_id, kind, text, owner, due, status, supersedes_artifact_id, segment_refs, created_at_ms, origin \
             FROM meeting_artifact WHERE id = ?1",
            params![artifact_id],
            map_artifact,
        )
        .optional()
        .map_err(Into::into)
    }

    fn get_entity(&self, entity_id: &str) -> Result<Option<EntityRow>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, meeting_id, name, kind, segment_refs, origin \
             FROM meeting_entity WHERE id = ?1",
            params![entity_id],
            map_entity,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_entities(&self, meeting_id: &str) -> Result<Vec<EntityRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, name, kind, segment_refs, origin \
             FROM meeting_entity WHERE meeting_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![meeting_id], map_entity)?;
        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// FTS lookup over entity names within one meeting.
    pub fn find_entities(&self, meeting_id: &str, name_query: &str) -> Result<Vec<EntityRow>> {
        let Some(match_query) = sanitize_fts5_query(name_query) else {
            return Ok(Vec::new());
        };
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT e.id, e.meeting_id, e.name, e.kind, e.segment_refs, e.origin \
             FROM entity_fts \
             JOIN meeting_entity e ON e.id = entity_fts.entity_id \
             WHERE entity_fts MATCH ?1 AND e.meeting_id = ?2 \
             ORDER BY rank",
        )?;
        let rows = stmt.query_map(params![match_query, meeting_id], map_entity)?;
        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Resolve `{direction, sequence}` refs to segment citations; refs that no
    /// longer resolve are dropped (artifact still renders, citation list shrinks).
    pub fn resolve_segment_refs(
        &self,
        meeting_id: &str,
        refs: &[SegmentRef],
    ) -> Result<Vec<SegmentCitation>> {
        if refs.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, started_at_ms FROM transcript_segment \
             WHERE meeting_id = ?1 AND direction = ?2 AND sequence = ?3",
        )?;
        let mut citations = Vec::with_capacity(refs.len());
        for segment_ref in refs {
            let hit = stmt
                .query_row(
                    params![meeting_id, segment_ref.direction, segment_ref.sequence],
                    |row| {
                        Ok(SegmentCitation {
                            segment_id: row.get(0)?,
                            direction: Some(segment_ref.direction.clone()),
                            started_at_ms: Some(row.get(1)?),
                            meeting_id: None,
                        })
                    },
                )
                .optional()?;
            if let Some(citation) = hit {
                citations.push(citation);
            }
        }
        Ok(citations)
    }
}

fn null_dangling_supersedes(tx: &rusqlite::Transaction<'_>, meeting_id: &str) -> Result<()> {
    tx.execute(
        "UPDATE meeting_artifact SET supersedes_artifact_id = NULL \
         WHERE meeting_id = ?1 AND supersedes_artifact_id IS NOT NULL \
           AND supersedes_artifact_id NOT IN \
               (SELECT id FROM meeting_artifact WHERE meeting_id = ?1)",
        params![meeting_id],
    )?;
    Ok(())
}

fn map_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactRow> {
    let refs_json: String = row.get(8)?;
    Ok(ArtifactRow {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        kind: row.get(2)?,
        text: row.get(3)?,
        owner: row.get(4)?,
        due: row.get(5)?,
        status: row.get(6)?,
        supersedes_artifact_id: row.get(7)?,
        segment_refs: serde_json::from_str(&refs_json).unwrap_or_default(),
        created_at_ms: row.get(9)?,
        origin: row.get(10)?,
    })
}

fn map_entity(row: &rusqlite::Row<'_>) -> rusqlite::Result<EntityRow> {
    let refs_json: String = row.get(4)?;
    Ok(EntityRow {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        name: row.get(2)?,
        kind: row.get(3)?,
        segment_refs: serde_json::from_str(&refs_json).unwrap_or_default(),
        origin: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::models::MeetingStatus;
    use crate::meeting::summary_schema::SummaryPointBlock;

    fn point(text: &str, sequence: i32) -> SummaryPoint {
        SummaryPoint {
            id: String::new(),
            text: text.to_string(),
            owner: None,
            due: None,
            segment_refs: vec![SegmentRef {
                direction: "outbound".to_string(),
                sequence,
            }],
            supersedes_index: None,
        }
    }

    fn brief() -> MeetingBriefSummary {
        MeetingBriefSummary {
            schema_version: 3,
            overview: SummaryPointBlock::default(),
            key_points: SummaryPointBlock::default(),
            decisions: SummaryPointBlock {
                points: vec![point("Use Redis Cluster", 0)],
            },
            action_items: SummaryPointBlock {
                points: vec![SummaryPoint {
                    owner: Some("Alice".to_string()),
                    due: Some("Friday".to_string()),
                    ..point("Prepare migration plan", 1)
                }],
            },
            open_questions: SummaryPointBlock {
                points: vec![point("Who owns rollback?", 2)],
            },
            entities: vec![
                crate::meeting::summary_schema::SummaryEntity {
                    name: "AUTH-123".to_string(),
                    kind: "ticket".to_string(),
                    segment_refs: vec![SegmentRef {
                        direction: "inbound".to_string(),
                        sequence: 0,
                    }],
                },
                crate::meeting::summary_schema::SummaryEntity {
                    name: "Redis".to_string(),
                    kind: "system".to_string(),
                    segment_refs: Vec::new(),
                },
            ],
            yesterday: SummaryPointBlock::default(),
            today: SummaryPointBlock::default(),
            blockers: SummaryPointBlock::default(),
        }
    }

    fn store_with_meeting() -> (MeetingStore, String) {
        let store = MeetingStore::open_in_memory().unwrap();
        let meeting = store
            .create_meeting("M", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        (store, meeting.id)
    }

    #[test]
    fn replace_persists_artifacts_and_entities() {
        let (store, meeting_id) = store_with_meeting();
        let count = store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        assert_eq!(count, 3);

        let decisions = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_DECISION), None)
            .unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].text, "Use Redis Cluster");
        assert_eq!(decisions[0].status, "proposed");

        let actions = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_ACTION_ITEM), None)
            .unwrap();
        assert_eq!(actions[0].owner.as_deref(), Some("Alice"));
        assert_eq!(actions[0].due.as_deref(), Some("Friday"));

        let entities = store.list_entities(&meeting_id).unwrap();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].name, "AUTH-123");
        assert_eq!(entities[0].kind, "ticket");
    }

    #[test]
    fn replace_is_idempotent_and_resets_status() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        let artifact_id = format!("{meeting_id}:{ARTIFACT_KIND_DECISION}:1");
        store
            .set_artifact_status(&artifact_id, "confirmed")
            .unwrap();

        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 2_000)
            .unwrap();
        let all = store.list_artifacts(&meeting_id, None, None).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].status, "proposed");
        assert_eq!(store.list_entities(&meeting_id).unwrap().len(), 2);
    }

    #[test]
    fn replace_resolves_decision_supersession_with_stable_ids() {
        let (store, meeting_id) = store_with_meeting();
        let mut summary = brief();
        summary.decisions.points.push(SummaryPoint {
            id: "de-2".to_string(),
            text: "Use PostgreSQL instead".to_string(),
            owner: None,
            due: None,
            segment_refs: Vec::new(),
            supersedes_index: Some(1),
        });

        store
            .replace_meeting_artifacts(&meeting_id, &summary, 1_000)
            .unwrap();

        let decisions = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_DECISION), None)
            .unwrap();
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].status, "superseded");
        let first_id = format!("{meeting_id}:decision:1");
        assert_eq!(
            decisions[1].supersedes_artifact_id.as_deref(),
            Some(first_id.as_str())
        );
        assert_eq!(decisions[1].status, "proposed");
    }

    #[test]
    fn status_transitions_and_validation() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        let artifact_id = format!("{meeting_id}:{ARTIFACT_KIND_ACTION_ITEM}:1");

        let updated = store
            .set_artifact_status(&artifact_id, "dismissed")
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, "dismissed");
        let dismissed = store
            .list_artifacts(&meeting_id, None, Some("dismissed"))
            .unwrap();
        assert_eq!(dismissed.len(), 1);

        assert!(store.set_artifact_status(&artifact_id, "bogus").is_err());
        assert!(store
            .set_artifact_status("m1:nope:1", "confirmed")
            .unwrap()
            .is_none());
    }

    #[test]
    fn entity_fts_finds_ticket_id() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();

        let hits = store.find_entities(&meeting_id, "AUTH-123").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "AUTH-123");
        assert!(store
            .find_entities(&meeting_id, "nonexistent-zzz")
            .unwrap()
            .is_empty());
        assert!(store.find_entities(&meeting_id, "!!!").unwrap().is_empty());
    }

    #[test]
    fn resolve_refs_maps_to_citations_and_skips_missing() {
        let (store, meeting_id) = store_with_meeting();
        for (index, sequence) in [1, 2, 3].into_iter().enumerate() {
            store
                .insert_segment_at_sequence(
                    &meeting_id,
                    "outbound",
                    sequence,
                    &format!("src {index}"),
                    "",
                    (sequence as i64) * 1_000,
                    (sequence as i64) * 1_000 + 500,
                    false,
                )
                .unwrap();
        }

        let refs = vec![
            SegmentRef {
                direction: "outbound".to_string(),
                sequence: 2,
            },
            SegmentRef {
                direction: "outbound".to_string(),
                sequence: 99,
            },
            SegmentRef {
                direction: "inbound".to_string(),
                sequence: 1,
            },
        ];
        let citations = store.resolve_segment_refs(&meeting_id, &refs).unwrap();
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].started_at_ms, Some(2_000));
        assert_eq!(citations[0].direction.as_deref(), Some("outbound"));
    }

    #[test]
    fn meeting_delete_cascades_artifacts_and_entities() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        store.delete_meeting(&meeting_id).unwrap();
        assert!(store
            .list_artifacts(&meeting_id, None, None)
            .unwrap()
            .is_empty());
        assert!(store.list_entities(&meeting_id).unwrap().is_empty());
        assert!(store
            .find_entities(&meeting_id, "AUTH-123")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn edit_artifact_rekeys_extracted_row_into_user_namespace() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        let extracted_id = format!("{meeting_id}:{ARTIFACT_KIND_DECISION}:1");

        let updated = store
            .update_artifact_content(&extracted_id, "Use Redis Cluster (edited)", None, None)
            .unwrap()
            .unwrap();
        // Re-keyed to the user namespace, origin flipped to 'edited'.
        assert_ne!(updated.id, extracted_id);
        assert!(updated.id.starts_with(&format!("{meeting_id}:decision:u")));
        assert_eq!(updated.origin, ARTIFACT_ORIGIN_EDITED);
        assert_eq!(updated.text, "Use Redis Cluster (edited)");

        // Original positional id is gone; the new id resolves.
        assert!(store.get_artifact(&extracted_id).unwrap().is_none());
        assert_eq!(
            store.get_artifact(&updated.id).unwrap().unwrap().text,
            "Use Redis Cluster (edited)"
        );
    }

    #[test]
    fn edit_keeps_id_when_row_is_already_user_owned() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        let extracted_id = format!("{meeting_id}:{ARTIFACT_KIND_DECISION}:1");
        let user_id = store
            .update_artifact_content(&extracted_id, "Keep id", None, None)
            .unwrap()
            .unwrap()
            .id;
        // A second edit on the already-user-owned row must keep the id.
        let again = store
            .update_artifact_content(&user_id, "Keep id 2", None, None)
            .unwrap()
            .unwrap();
        assert_eq!(again.id, user_id);
        assert_eq!(again.origin, ARTIFACT_ORIGIN_EDITED);
    }

    #[test]
    fn edit_repoints_supersession_links_from_other_rows() {
        let (store, meeting_id) = store_with_meeting();
        let mut summary = brief();
        summary.decisions.points.push(SummaryPoint {
            id: "de-2".to_string(),
            text: "Use PostgreSQL instead".to_string(),
            owner: None,
            due: None,
            segment_refs: Vec::new(),
            supersedes_index: Some(1),
        });
        store
            .replace_meeting_artifacts(&meeting_id, &summary, 1_000)
            .unwrap();
        let superseded_id = format!("{meeting_id}:decision:1");
        let newly_keyed = store
            .update_artifact_content(&superseded_id, "Edited decision", None, None)
            .unwrap()
            .unwrap();
        let survivor = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_DECISION), None)
            .unwrap();
        let postgres = survivor
            .iter()
            .find(|a| a.text == "Use PostgreSQL instead")
            .unwrap();
        // Supersession link now follows the re-keyed id.
        assert_eq!(
            postgres.supersedes_artifact_id.as_deref(),
            Some(newly_keyed.id.as_str())
        );
    }

    #[test]
    fn edit_artifact_validates_and_rejects_missing() {
        let (store, meeting_id) = store_with_meeting();
        assert!(store
            .update_artifact_content("no-such-id", "x", None, None)
            .unwrap()
            .is_none());
        let artifact_id = format!("{meeting_id}:{ARTIFACT_KIND_ACTION_ITEM}:1");
        assert!(store
            .update_artifact_content(&artifact_id, "   ", None, None)
            .is_err());
    }

    #[test]
    fn create_and_delete_manual_artifact() {
        let (store, meeting_id) = store_with_meeting();
        let created = store
            .create_artifact(
                &meeting_id,
                ARTIFACT_KIND_ACTION_ITEM,
                "Ship auth service",
                Some("Nha"),
                Some("Fri"),
                2_000,
            )
            .unwrap();
        assert_eq!(created.origin, ARTIFACT_ORIGIN_MANUAL);
        assert_eq!(created.status, "proposed");
        assert_eq!(created.owner.as_deref(), Some("Nha"));
        assert_eq!(created.due.as_deref(), Some("Fri"));
        assert!(created
            .id
            .starts_with(&format!("{meeting_id}:action_item:u")));
        assert!(created.segment_refs.is_empty());

        assert!(store.delete_artifact(&created.id).unwrap());
        assert!(store.get_artifact(&created.id).unwrap().is_none());
        assert!(!store.delete_artifact(&created.id).unwrap());
    }

    #[test]
    fn create_artifact_rejects_bad_kind_and_empty() {
        let (store, meeting_id) = store_with_meeting();
        assert!(store
            .create_artifact(&meeting_id, "bogus", "x", None, None, 0)
            .is_err());
        assert!(store
            .create_artifact(&meeting_id, ARTIFACT_KIND_DECISION, "  ", None, None, 0)
            .is_err());
    }

    #[test]
    fn replace_preserves_user_rows_and_nulls_dangling_supersedes() {
        let (store, meeting_id) = store_with_meeting();
        let mut summary = brief();
        summary.decisions.points.push(SummaryPoint {
            id: "de-2".to_string(),
            text: "Use PostgreSQL instead".to_string(),
            owner: None,
            due: None,
            segment_refs: Vec::new(),
            supersedes_index: Some(1),
        });
        store
            .replace_meeting_artifacts(&meeting_id, &summary, 1_000)
            .unwrap();

        // Edit decision 1 -> re-keyed + origin edited. Manually add another row.
        let superseded_id = format!("{meeting_id}:decision:1");
        let preserved_id = store
            .update_artifact_content(&superseded_id, "Edited decision", None, None)
            .unwrap()
            .unwrap()
            .id;
        let manual = store
            .create_artifact(
                &meeting_id,
                ARTIFACT_KIND_DECISION,
                "Manual decision",
                None,
                None,
                2_000,
            )
            .unwrap();

        // Regenerate wipes extracted rows only; surviving edit referenced the
        // extracted decision:2 which is deleted, so its supersedes link nulls.
        store
            .replace_meeting_artifacts(&meeting_id, &brief_with_two_decisions(), 9_000)
            .unwrap();

        let all = store.list_artifacts(&meeting_id, None, None).unwrap();
        let preserved = all.iter().find(|a| a.id == preserved_id).unwrap();
        assert_eq!(preserved.origin, ARTIFACT_ORIGIN_EDITED);
        assert_eq!(preserved.text, "Edited decision");
        assert_eq!(preserved.supersedes_artifact_id, None);
        let manual_after = all.iter().find(|a| a.id == manual.id).unwrap();
        assert_eq!(manual_after.origin, ARTIFACT_ORIGIN_MANUAL);
        // Fresh extracted rows came back (with positional ids).
        assert!(all.iter().any(|a| a.text == "Fresh A"));
        assert!(all.iter().any(|a| a.text == "Fresh B"));
    }

    #[test]
    fn entity_rename_updates_fts_and_suppresses_old_name_and_dedupes_new_name() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        let entity_id = format!("{meeting_id}:entity:Redis");
        assert!(store
            .update_entity_content(&entity_id, "Redis Cluster", "system")
            .unwrap()
            .is_some());

        // FTS tokenizes "Redis Cluster" → both "Redis" and "Cluster" tokens,
        // so querying "Redis" MATCHES the renamed row. Check exact-name state.
        let names = || {
            store
                .list_entities(&meeting_id)
                .unwrap()
                .into_iter()
                .map(|e| e.name)
                .collect::<Vec<_>>()
        };
        assert!(!names().contains(&"Redis".to_string()));
        assert_eq!(names().iter().filter(|n| *n == "Redis Cluster").count(), 1);
        assert_eq!(
            store.get_entity(&entity_id).unwrap().unwrap().origin,
            ARTIFACT_ORIGIN_EDITED
        );

        // Regenerate with the old name: INSERT OR IGNORE no-ops (id collision),
        // so the renamed entity survives and the old name stays suppressed.
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 9_000)
            .unwrap();
        let e = store.get_entity(&entity_id).unwrap().unwrap();
        assert_eq!(e.name, "Redis Cluster");
        assert!(!names().contains(&"Redis".to_string()));

        // Regenerate with the NEW name but different id: name-column dedupe
        // skips it, keeping exactly one "Redis Cluster" row (gap 2).
        let mut newer = brief();
        newer.entities = vec![crate::meeting::summary_schema::SummaryEntity {
            name: "Redis Cluster".to_string(),
            kind: "system".to_string(),
            segment_refs: Vec::new(),
        }];
        store
            .replace_meeting_artifacts(&meeting_id, &newer, 10_000)
            .unwrap();
        assert_eq!(names().iter().filter(|n| *n == "Redis Cluster").count(), 1);
        assert_eq!(
            store.get_entity(&entity_id).unwrap().unwrap().origin,
            ARTIFACT_ORIGIN_EDITED
        );
    }

    #[test]
    fn create_entity_conflict_returns_none_and_delete_cleans_fts() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();

        // Rename to a free name first (id stays Redis), then test conflict on a
        // name-keyed insert of an existing name.
        let created = store
            .create_entity(&meeting_id, "Zoom Bot", "person", 2_000)
            .unwrap()
            .expect("Zoom Bot should insert");
        assert_eq!(created.origin, ARTIFACT_ORIGIN_MANUAL);
        // Creating the same name again conflicts.
        assert!(store
            .create_entity(&meeting_id, "Zoom Bot", "person", 3_000)
            .unwrap()
            .is_none());

        assert!(store.delete_entity(&created.id).unwrap());
        assert!(store
            .find_entities(&meeting_id, "Zoom Bot")
            .unwrap()
            .is_empty());
        assert!(!store.delete_entity(&created.id).unwrap());
    }

    #[test]
    fn replace_skips_extracted_artifact_matching_edited_survivor() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();

        // Edit the action item to add a confirmation suffix (survives regenerate).
        let extracted_action_id = format!("{meeting_id}:{ARTIFACT_KIND_ACTION_ITEM}:1");
        let preserved_id = store
            .update_artifact_content(
                &extracted_action_id,
                "Prepare migration plan (confirmed by Alice)",
                None,
                None,
            )
            .unwrap()
            .unwrap()
            .id;

        // Regenerate with the same brief — fresh "Prepare migration plan" is a
        // containment substring of the surviving text (21 chars ≥ 10) → deduped.
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 9_000)
            .unwrap();

        let all = store.list_artifacts(&meeting_id, None, None).unwrap();
        let actions = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_ACTION_ITEM), None)
            .unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, preserved_id);
        assert_eq!(actions[0].origin, ARTIFACT_ORIGIN_EDITED);
        // Other kinds still re-extracted because their text didn't match.
        assert!(all.iter().any(|a| a.text == "Use Redis Cluster"));
        assert!(all.iter().any(|a| a.text == "Who owns rollback?"));
    }

    #[test]
    fn replace_skips_extracted_artifact_matching_manual_one() {
        let (store, meeting_id) = store_with_meeting();
        let manual = store
            .create_artifact(
                &meeting_id,
                ARTIFACT_KIND_DECISION,
                "Use PostgreSQL instead",
                None,
                None,
                2_000,
            )
            .unwrap();

        // Only the matching decision — other kinds from brief still insert.
        let mut summary = brief();
        summary.decisions.points = vec![SummaryPoint {
            id: "de-2".to_string(),
            text: "Use PostgreSQL instead".to_string(),
            owner: None,
            due: None,
            segment_refs: Vec::new(),
            supersedes_index: None,
        }];

        store
            .replace_meeting_artifacts(&meeting_id, &summary, 9_000)
            .unwrap();
        let decisions = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_DECISION), None)
            .unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].id, manual.id);
        assert_eq!(decisions[0].origin, ARTIFACT_ORIGIN_MANUAL);
    }

    #[test]
    fn replace_does_not_dedupe_short_substring_hits() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();

        let extracted_action_id = format!("{meeting_id}:{ARTIFACT_KIND_ACTION_ITEM}:1");
        let _preserved_id = store
            .update_artifact_content(&extracted_action_id, "Ship", None, None)
            .unwrap()
            .unwrap()
            .id;

        // "ship" (4 chars) is contained in "Prepare migration plan"? No.
        // Replace brief action with "Ship product" — "Ship" (4 chars < 10)
        // should NOT be deduped against "Ship".
        let mut summary = brief();
        summary.action_items.points = vec![SummaryPoint {
            id: "a1".to_string(),
            text: "Ship product".to_string(),
            owner: None,
            due: None,
            segment_refs: Vec::new(),
            supersedes_index: None,
        }];
        store
            .replace_meeting_artifacts(&meeting_id, &summary, 9_000)
            .unwrap();

        let actions = store
            .list_artifacts(&meeting_id, Some(ARTIFACT_KIND_ACTION_ITEM), None)
            .unwrap();
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn create_entity_conflicts_on_renamed_name_with_different_id() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        let entity_id = format!("{meeting_id}:entity:Redis");
        store
            .update_entity_content(&entity_id, "Redis Cluster", "system")
            .unwrap();

        // Row id is still m:entity:Redis, name is "Redis Cluster".
        // create_entity with "Redis Cluster" must conflict by normalized name.
        assert!(store
            .create_entity(&meeting_id, "Redis Cluster", "system", 3_000)
            .unwrap()
            .is_none());
    }

    #[test]
    fn update_entity_conflicts_on_normalized_name_of_another_row() {
        let (store, meeting_id) = store_with_meeting();
        store
            .replace_meeting_artifacts(&meeting_id, &brief(), 1_000)
            .unwrap();
        // Brief has "Redis"; add a second entity.
        let zoom = store
            .create_entity(&meeting_id, "Zoom Bot", "person", 2_000)
            .unwrap()
            .expect("Zoom Bot should insert");

        // Case-variant of an existing name must fail (gap 2 / Bugbot).
        let err = store
            .update_entity_content(&zoom.id, "redis", "system")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("entity already exists"),
            "expected name conflict, got: {err}"
        );
        // Unchanged on conflict.
        let still = store.get_entity(&zoom.id).unwrap().unwrap();
        assert_eq!(still.name, "Zoom Bot");
        assert_eq!(still.kind, "person");

        // Own name (kind-only / same-name rename) still succeeds.
        let updated = store
            .update_entity_content(&zoom.id, "Zoom Bot", "tool")
            .unwrap()
            .unwrap();
        assert_eq!(updated.kind, "tool");
        assert_eq!(updated.name, "Zoom Bot");
    }

    fn brief_with_two_decisions() -> MeetingBriefSummary {
        let mut b = brief();
        b.decisions.points = vec![
            point("Fresh A", 3),
            SummaryPoint {
                id: "d2".to_string(),
                text: "Fresh B".to_string(),
                owner: None,
                due: None,
                segment_refs: Vec::new(),
                supersedes_index: None,
            },
        ];
        b
    }
}
