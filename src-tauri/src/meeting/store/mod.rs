mod artifacts;
mod audio_chunks;
mod folders;
mod fts;
mod meetings;
mod paths;
mod segments;
mod summaries;

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub use artifacts::{
    ArtifactRow, EntityRow, ARTIFACT_KIND_ACTION_ITEM, ARTIFACT_KIND_DECISION,
    ARTIFACT_KIND_OPEN_QUESTION, ARTIFACT_STATUSES,
};
pub use audio_chunks::{AudioChunkRow, MissingAudioSpan, VerifyMeetingAudioResult};
pub(crate) use fts::sanitize_fts5_query;
pub use paths::{
    chunk_path, default_recordings_dir, format_meeting_title, meeting_db_path,
    meeting_recordings_dir, resolve_recordings_base, wall_ms,
};

const SCHEMA_INIT: &str = include_str!("../migrations/001_initial.sql");

pub struct MeetingStore {
    conn: Mutex<Connection>,
}

fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_INIT).context("run schema init")?;
    Ok(())
}

fn apply_connection_pragmas(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .context("enable foreign keys")?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("enable WAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .context("set busy_timeout")?;
    Ok(())
}

impl MeetingStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create meetings db parent dir")?;
        }
        let conn = Connection::open(path).context("open meetings db")?;
        apply_connection_pragmas(&conn)?;
        apply_migrations(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        // Best-effort one-pass fill for pre-byte_size rows; ignore manual-delete drift.
        let _ = store.backfill_audio_chunk_byte_sizes();
        let _ = store.backfill_transcript_fts();
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("open in-memory db")?;
        apply_connection_pragmas(&conn)?;
        apply_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        crate::meeting::lock_poison_recover(&self.conn, "meeting store")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::models::MeetingStatus;

    #[test]
    fn format_meeting_title_uses_local_datetime_without_prefix() {
        let title = format_meeting_title(1_700_000_000_000);
        assert!(!title.starts_with("Meeting "));
        assert!(!title.contains('·'));
    }

    #[test]
    fn reorder_folders_updates_sort_order() {
        let store = MeetingStore::open_in_memory().unwrap();
        let f1 = store.create_folder("A", None).unwrap();
        let f2 = store.create_folder("B", None).unwrap();
        let f3 = store.create_folder("C", None).unwrap();
        let reordered = store
            .reorder_folders(&[f3.id.clone(), f1.id.clone(), f2.id.clone()])
            .unwrap();
        assert_eq!(
            reordered.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            vec![f3.id.as_str(), f1.id.as_str(), f2.id.as_str()],
        );
        assert_eq!(reordered[0].sort_order, 0);
        assert_eq!(reordered[2].sort_order, 2);
    }

    #[test]
    fn creates_folder_and_meeting() {
        let store = MeetingStore::open_in_memory().unwrap();
        let folder = store.create_folder("Q1", None).unwrap();
        assert_eq!(folder.name, "Q1");
        let meeting = store
            .create_meeting(
                "Standup",
                Some(&folder.id),
                "vi",
                "en",
                "interpreter",
                MeetingStatus::Live,
            )
            .unwrap();
        assert_eq!(meeting.title, "Standup");
        assert_eq!(meeting.status, MeetingStatus::Live);
        assert_eq!(meeting.session_mode, "interpreter");
    }

    #[test]
    fn create_meeting_persists_notes_session_mode() {
        let store = MeetingStore::open_in_memory().unwrap();
        let meeting = store
            .create_meeting("Notes", None, "vi", "vi", "notes", MeetingStatus::Live)
            .unwrap();
        assert_eq!(meeting.session_mode, "notes");
        let loaded = store.get_meeting(&meeting.id, false).unwrap();
        assert_eq!(loaded.session_mode, "notes");
    }

    #[test]
    fn lists_meetings_ordered_by_started_desc() {
        let store = MeetingStore::open_in_memory().unwrap();
        store
            .create_meeting("Old", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        store
            .create_meeting("New", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        let list = store.list_meetings(None, 10, 0).unwrap();
        assert_eq!(list.meetings.len(), 2);
        assert_eq!(list.meetings[0].title, "New");
    }

    #[test]
    fn move_meeting_updates_folder_id() {
        let store = MeetingStore::open_in_memory().unwrap();
        let f1 = store.create_folder("A", None).unwrap();
        let f2 = store.create_folder("B", None).unwrap();
        let m = store
            .create_meeting(
                "M",
                Some(&f1.id),
                "vi",
                "en",
                "interpreter",
                MeetingStatus::Ended,
            )
            .unwrap();
        let moved = store.move_meeting(&m.id, Some(&f2.id)).unwrap();
        assert_eq!(moved.folder_id.as_deref(), Some(f2.id.as_str()));
    }

    #[test]
    fn insert_segment_increments_sequence() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Live)
            .unwrap();
        let s1 = store
            .insert_segment(&m.id, "inbound", "hello", "xin chào", 0, 100, false)
            .unwrap();
        let s2 = store
            .insert_segment(&m.id, "inbound", "world", "thế giới", 100, 200, false)
            .unwrap();
        assert_eq!(s1.sequence, 1);
        assert_eq!(s2.sequence, 2);
    }

    #[test]
    fn list_segments_tail_returns_last_n_ascending() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Live)
            .unwrap();
        for i in 1..=5 {
            store
                .insert_segment(
                    &m.id,
                    "inbound",
                    &format!("s{i}"),
                    &format!("t{i}"),
                    i * 10,
                    i * 10 + 5,
                    false,
                )
                .unwrap();
        }
        let resp = store
            .list_segments(&m.id, Some("inbound"), None, None, true, 3)
            .unwrap();
        assert_eq!(resp.segments.len(), 3);
        assert_eq!(resp.segments[0].sequence, 3);
        assert_eq!(resp.segments[2].sequence, 5);
        assert!(resp.has_more_older);
    }

    #[test]
    fn list_segments_before_returns_older_page() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Live)
            .unwrap();
        for i in 1..=4 {
            store
                .insert_segment(
                    &m.id,
                    "outbound",
                    &format!("s{i}"),
                    &format!("t{i}"),
                    i * 10,
                    i * 10 + 5,
                    false,
                )
                .unwrap();
        }
        let resp = store
            .list_segments(&m.id, Some("outbound"), None, Some(3), false, 2)
            .unwrap();
        assert_eq!(resp.segments.len(), 2);
        assert_eq!(resp.segments[0].sequence, 1);
        assert_eq!(resp.segments[1].sequence, 2);
        assert!(!resp.has_more_older);
    }

    #[test]
    fn list_segments_tail_empty_meeting() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Live)
            .unwrap();
        let resp = store
            .list_segments(&m.id, Some("inbound"), None, None, true, 10)
            .unwrap();
        assert!(resp.segments.is_empty());
        assert!(!resp.has_more_older);
    }

    #[test]
    fn get_segment_neighbors() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        let s1 = store
            .insert_segment(&m.id, "inbound", "a", "a", 0, 1, false)
            .unwrap();
        let s2 = store
            .insert_segment(&m.id, "inbound", "b", "b", 1, 2, false)
            .unwrap();
        let s3 = store
            .insert_segment(&m.id, "inbound", "c", "c", 2, 3, false)
            .unwrap();
        let n = store.get_segment_neighbors(&m.id, &s2.id).unwrap();
        assert_eq!(n.prev.as_ref().map(|s| s.id.as_str()), Some(s1.id.as_str()));
        assert_eq!(n.next.as_ref().map(|s| s.id.as_str()), Some(s3.id.as_str()));
    }

    #[test]
    fn audio_chunk_insert_list_has() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Live)
            .unwrap();
        assert!(!store.has_audio_chunks(&m.id).unwrap());
        store
            .insert_audio_chunk(AudioChunkRow {
                id: "c1".into(),
                meeting_id: m.id.clone(),
                direction: "inbound".into(),
                sequence: 0,
                file_path: "/tmp/m/inbound/0.opus".into(),
                duration_ms: 30_000,
                sample_rate: 16_000,
                started_at_ms: 0,
                byte_size: 1_024,
            })
            .unwrap();
        assert!(store.has_audio_chunks(&m.id).unwrap());
        let rows = store.list_audio_chunks(&m.id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].file_path, "/tmp/m/inbound/0.opus");
        assert_eq!(rows[0].byte_size, 1_024);
        assert_eq!(store.total_audio_byte_size().unwrap(), 1_024);
        let paths = store.list_audio_chunk_paths(&m.id).unwrap();
        assert_eq!(paths, vec!["/tmp/m/inbound/0.opus".to_string()]);
    }

    #[test]
    fn verify_meeting_audio_zeros_missing_and_keeps_present() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "interpreter", MeetingStatus::Live)
            .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "meetral-verify-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let present_path = dir.join("0.opus");
        std::fs::write(&present_path, [0u8; 50]).unwrap();

        store
            .insert_audio_chunk(AudioChunkRow {
                id: "ok".into(),
                meeting_id: m.id.clone(),
                direction: "inbound".into(),
                sequence: 0,
                file_path: present_path.to_string_lossy().to_string(),
                duration_ms: 1_000,
                sample_rate: 16_000,
                started_at_ms: 0,
                byte_size: 999,
            })
            .unwrap();
        store
            .insert_audio_chunk(AudioChunkRow {
                id: "gone".into(),
                meeting_id: m.id.clone(),
                direction: "inbound".into(),
                sequence: 1,
                file_path: dir.join("missing.opus").to_string_lossy().to_string(),
                duration_ms: 1_000,
                sample_rate: 16_000,
                started_at_ms: 1_000,
                byte_size: 400,
            })
            .unwrap();

        let verified = store.verify_meeting_audio_chunks(&m.id).unwrap();
        assert_eq!(verified.missing_count, 1);
        assert_eq!(verified.recorded_count, 2);
        assert_eq!(verified.missing.len(), 1);
        assert_eq!(verified.chunks.len(), 1);
        assert_eq!(verified.chunks[0].id, "ok");
        assert_eq!(verified.chunks[0].byte_size, 50);
        assert_eq!(store.total_audio_byte_size().unwrap(), 50);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fts_vietnamese_diacritics_and_bilingual() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting(
                "Standup",
                None,
                "vi",
                "en",
                "interpreter",
                MeetingStatus::Ended,
            )
            .unwrap();
        let seg = store
            .insert_segment(
                &m.id,
                "inbound",
                "Chúng ta đã quyết định ngân sách",
                "We decided on the budget",
                0,
                100,
                false,
            )
            .unwrap();

        // Tone marks fold (ế→e); letter đ is not a diacritic — query with diacritics or ASCII tones only.
        let by_vi_ascii_tones = store
            .search_segments("quyet", Some(&m.id), None, Some(10))
            .unwrap();
        assert_eq!(by_vi_ascii_tones.len(), 1);
        assert_eq!(by_vi_ascii_tones[0].segment_id, seg.id);

        let by_vi_native = store
            .search_segments("quyết định", Some(&m.id), None, Some(10))
            .unwrap();
        assert_eq!(by_vi_native.len(), 1);
        assert_eq!(by_vi_native[0].segment_id, seg.id);

        let by_en = store
            .search_segments("budget", Some(&m.id), None, Some(10))
            .unwrap();
        assert_eq!(by_en.len(), 1);
        assert_eq!(by_en[0].segment_id, seg.id);

        let meetings = store.search_meetings("budget", None, Some(10)).unwrap();
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].meeting_id, m.id);
        assert_eq!(meetings[0].best_segment_id, seg.id);
        assert_eq!(meetings[0].started_at_ms, Some(0));
    }

    #[test]
    fn fts_french_diacritics_and_meeting_filter() {
        let store = MeetingStore::open_in_memory().unwrap();
        let a = store
            .create_meeting("A", None, "fr", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        let b = store
            .create_meeting("B", None, "fr", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        store
            .insert_segment(
                &a.id,
                "inbound",
                "Décision finale",
                "Final decision",
                0,
                1,
                false,
            )
            .unwrap();
        store
            .insert_segment(&b.id, "inbound", "autre sujet", "other topic", 0, 1, false)
            .unwrap();

        let hits = store
            .search_segments("decision", Some(&a.id), None, None)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].meeting_id, a.id);

        let leaked = store
            .search_segments("decision", Some(&b.id), None, None)
            .unwrap();
        assert!(leaked.is_empty());
    }

    #[test]
    fn fts_folder_filter_and_empty_query() {
        let store = MeetingStore::open_in_memory().unwrap();
        let folder = store.create_folder("Q1", None).unwrap();
        let in_folder = store
            .create_meeting(
                "In",
                Some(&folder.id),
                "vi",
                "en",
                "interpreter",
                MeetingStatus::Ended,
            )
            .unwrap();
        let out = store
            .create_meeting("Out", None, "vi", "en", "interpreter", MeetingStatus::Ended)
            .unwrap();
        store
            .insert_segment(
                &in_folder.id,
                "inbound",
                "alpha topic",
                "alpha topic",
                0,
                1,
                false,
            )
            .unwrap();
        store
            .insert_segment(
                &out.id,
                "inbound",
                "alpha topic",
                "alpha topic",
                0,
                1,
                false,
            )
            .unwrap();

        let scoped = store
            .search_segments("alpha", None, Some(&folder.id), None)
            .unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].meeting_id, in_folder.id);

        assert!(store
            .search_segments("   ", None, None, None)
            .unwrap()
            .is_empty());
        assert!(store.search_meetings("***", None, None).unwrap().is_empty());
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
            rusqlite::params![table, column],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn object_exists(conn: &Connection, name: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn assert_final_schema(conn: &Connection) {
        assert_eq!(column_exists(conn, "meeting_record", "session_mode"), 1);
        assert_eq!(column_exists(conn, "audio_chunk", "byte_size"), 1);
        assert_eq!(object_exists(conn, "transcript_fts"), 1);
        assert_eq!(object_exists(conn, "meeting_artifact"), 1);
        assert_eq!(
            column_exists(conn, "meeting_artifact", "supersedes_artifact_id"),
            1
        );
        assert_eq!(column_exists(conn, "meeting_artifact", "origin"), 1);
        assert_eq!(object_exists(conn, "meeting_entity"), 1);
        assert_eq!(column_exists(conn, "meeting_entity", "origin"), 1);
        assert_eq!(object_exists(conn, "entity_fts"), 1);
    }

    #[test]
    fn schema_init_is_idempotent() {
        let store = MeetingStore::open_in_memory().unwrap();
        assert_final_schema(&store.conn());
        apply_migrations(&store.conn()).unwrap();
        assert_final_schema(&store.conn());
    }

    #[test]
    fn fts_backfill_and_delete_cascade() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("T", None, "vi", "en", "notes", MeetingStatus::Ended)
            .unwrap();
        let id = store
            .insert_segment_without_fts(
                &m.id,
                "inbound",
                "backfill marker xyz",
                "backfill marker xyz",
            )
            .unwrap();
        assert_eq!(store.fts_row_count_for_segment(&id).unwrap(), 0);
        assert_eq!(store.backfill_transcript_fts().unwrap(), 1);
        assert_eq!(store.fts_row_count_for_segment(&id).unwrap(), 1);
        let hits = store
            .search_segments("marker", Some(&m.id), None, None)
            .unwrap();
        assert_eq!(hits.len(), 1);

        store.delete_meeting(&m.id).unwrap();
        assert_eq!(store.fts_row_count_for_segment(&id).unwrap(), 0);
    }

    #[test]
    fn list_last_segments_orders_and_limits() {
        let store = MeetingStore::open_in_memory().unwrap();
        let m = store
            .create_meeting("Tail", None, "vi", "en", "notes", MeetingStatus::Live)
            .unwrap();
        for (i, text) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            let start = (i as i64) * 1_000;
            store
                .insert_segment(&m.id, "inbound", text, text, start, start + 100, false)
                .unwrap();
        }
        let last3 = store.list_last_segments(&m.id, 3).unwrap();
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].source_text, "c");
        assert_eq!(last3[1].source_text, "d");
        assert_eq!(last3[2].source_text, "e");
        // Chronological (ascending started_at_ms).
        assert!(last3[0].started_at_ms < last3[1].started_at_ms);
        assert!(last3[1].started_at_ms < last3[2].started_at_ms);

        let empty = store.list_last_segments("no-such-meeting", 15).unwrap();
        assert!(empty.is_empty());
    }
}
