use anyhow::{Context, Result};
use serde::Serialize;

use super::MeetingStore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioChunkRow {
    pub id: String,
    pub meeting_id: String,
    pub direction: String,
    pub sequence: i64,
    pub file_path: String,
    pub duration_ms: i64,
    pub sample_rate: i64,
    pub started_at_ms: i64,
    /// On-disk file size at insert (or backfill); used for Settings disk-usage SUM.
    pub byte_size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingAudioSpan {
    pub started_at_ms: i64,
    pub duration_ms: i64,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyMeetingAudioResult {
    pub chunks: Vec<AudioChunkRow>,
    pub missing_count: u32,
    /// Total DB chunk rows before filtering missing files.
    pub recorded_count: u32,
    pub missing: Vec<MissingAudioSpan>,
}

impl MeetingStore {
    pub fn insert_audio_chunk(&self, row: AudioChunkRow) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO audio_chunk (id, meeting_id, direction, sequence, file_path, duration_ms, sample_rate, started_at_ms, byte_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                row.id,
                row.meeting_id,
                row.direction,
                row.sequence,
                row.file_path,
                row.duration_ms,
                row.sample_rate,
                row.started_at_ms,
                row.byte_size,
            ],
        )
        .context("insert audio_chunk")?;
        Ok(())
    }

    pub fn list_audio_chunks(&self, meeting_id: &str) -> Result<Vec<AudioChunkRow>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, meeting_id, direction, sequence, file_path, duration_ms, sample_rate, started_at_ms, byte_size
                 FROM audio_chunk WHERE meeting_id = ?1
                 ORDER BY direction ASC, sequence ASC",
            )
            .context("prepare list audio_chunk")?;
        let rows = stmt
            .query_map(rusqlite::params![meeting_id], |r| {
                Ok(AudioChunkRow {
                    id: r.get(0)?,
                    meeting_id: r.get(1)?,
                    direction: r.get(2)?,
                    sequence: r.get(3)?,
                    file_path: r.get(4)?,
                    duration_ms: r.get(5)?,
                    sample_rate: r.get(6)?,
                    started_at_ms: r.get(7)?,
                    byte_size: r.get(8)?,
                })
            })
            .context("query audio_chunk")?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("audio_chunk row")?);
        }
        Ok(out)
    }

    pub fn has_audio_chunks(&self, meeting_id: &str) -> Result<bool> {
        let conn = self.conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_chunk WHERE meeting_id = ?1",
                rusqlite::params![meeting_id],
                |r| r.get(0),
            )
            .context("count audio_chunk")?;
        Ok(n > 0)
    }

    /// Absolute file paths for a meeting (for FS cleanup before CASCADE delete).
    pub fn list_audio_chunk_paths(&self, meeting_id: &str) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT file_path FROM audio_chunk WHERE meeting_id = ?1")
            .context("prepare audio paths")?;
        let rows = stmt
            .query_map(rusqlite::params![meeting_id], |r| r.get::<_, String>(0))
            .context("query audio paths")?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("path row")?);
        }
        Ok(out)
    }

    /// Sum of stored `byte_size` values (no filesystem walk).
    pub fn total_audio_byte_size(&self) -> Result<u64> {
        let conn = self.conn();
        let total: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(byte_size), 0) FROM audio_chunk",
                [],
                |r| r.get(0),
            )
            .context("sum audio_chunk.byte_size")?;
        Ok(u64::try_from(total.max(0)).unwrap_or(0))
    }

    /// Check chunk files on disk for a meeting: zero `byte_size` when missing;
    /// refresh size when present but differs. Returns only playable (existing) rows.
    pub fn verify_meeting_audio_chunks(
        &self,
        meeting_id: &str,
    ) -> Result<VerifyMeetingAudioResult> {
        let rows = self.list_audio_chunks(meeting_id)?;
        let recorded_count = rows.len() as u32;
        let mut present = Vec::with_capacity(rows.len());
        let mut missing = Vec::new();

        for mut row in rows {
            match std::fs::metadata(&row.file_path) {
                Ok(meta) if meta.is_file() => {
                    let len = i64::try_from(meta.len()).unwrap_or(i64::MAX);
                    if row.byte_size != len {
                        self.set_audio_chunk_byte_size(&row.id, len)?;
                        row.byte_size = len;
                    }
                    present.push(row);
                }
                _ => {
                    if row.byte_size != 0 {
                        self.set_audio_chunk_byte_size(&row.id, 0)?;
                    }
                    missing.push(MissingAudioSpan {
                        started_at_ms: row.started_at_ms,
                        duration_ms: row.duration_ms,
                        direction: row.direction,
                    });
                }
            }
        }

        let missing_count = missing.len() as u32;
        Ok(VerifyMeetingAudioResult {
            chunks: present,
            missing_count,
            recorded_count,
            missing,
        })
    }

    fn set_audio_chunk_byte_size(&self, id: &str, byte_size: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE audio_chunk SET byte_size = ?1 WHERE id = ?2",
            rusqlite::params![byte_size, id],
        )
        .context("update audio_chunk.byte_size")?;
        Ok(())
    }

    /// One-shot fill for rows created before `byte_size` existed (or default 0).
    pub fn backfill_audio_chunk_byte_sizes(&self) -> Result<usize> {
        let pending: Vec<(String, String)> = {
            let conn = self.conn();
            let mut stmt = conn
                .prepare("SELECT id, file_path FROM audio_chunk WHERE byte_size = 0")
                .context("prepare byte_size backfill")?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .context("query byte_size backfill")?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.context("backfill row")?);
            }
            out
        };
        if pending.is_empty() {
            return Ok(0);
        }

        let mut updated = 0usize;
        let conn = self.conn();
        for (id, path) in pending {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let len = i64::try_from(meta.len()).unwrap_or(i64::MAX);
            let n = conn
                .execute(
                    "UPDATE audio_chunk SET byte_size = ?1 WHERE id = ?2 AND byte_size = 0",
                    rusqlite::params![len, id],
                )
                .context("update audio_chunk.byte_size")?;
            updated += n;
        }
        Ok(updated)
    }
}
