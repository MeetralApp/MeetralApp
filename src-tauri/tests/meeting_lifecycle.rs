//! Integration test: meeting lifecycle on a real SQLite file (WAL + schema init).
//! Headless — `MeetingStore` needs no Tauri `AppHandle`.

use meetral_lib::meeting::models::MeetingStatus;
use meetral_lib::meeting::MeetingStore;

#[test]
fn meeting_lifecycle_persists_across_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("meetings.db");

    let meeting_id = {
        let store = MeetingStore::open(&db_path).expect("open db");
        let meeting = store
            .create_meeting(
                "Standup",
                None,
                "vi",
                "en",
                "interpreter",
                MeetingStatus::Live,
            )
            .expect("create meeting");

        for i in 1..=3_i64 {
            store
                .insert_segment(
                    &meeting.id,
                    "outbound",
                    &format!("xin chào {i}"),
                    &format!("hello {i}"),
                    i * 10,
                    i * 10 + 5,
                    false,
                )
                .expect("insert outbound segment");
        }
        for i in 1..=2_i64 {
            store
                .insert_segment(
                    &meeting.id,
                    "inbound",
                    &format!("budget topic {i}"),
                    &format!("chủ đề ngân sách {i}"),
                    i * 20,
                    i * 20 + 5,
                    false,
                )
                .expect("insert inbound segment");
        }

        let tail = store
            .list_segments(&meeting.id, Some("inbound"), None, None, true, 10)
            .expect("tail segments");
        assert_eq!(tail.segments.len(), 2);
        assert_eq!(tail.segments[0].sequence, 1);

        let hits = store
            .search_segments("budget", Some(&meeting.id), None, Some(10))
            .expect("fts search");
        assert_eq!(hits.len(), 2);

        let reserved = store.search_segments("AND OR NOT", Some(&meeting.id), None, Some(10));
        assert!(
            reserved.is_ok(),
            "reserved-word query errored: {reserved:?}"
        );

        let ended = store.end_meeting(&meeting.id).expect("end meeting");
        assert_eq!(ended.status, MeetingStatus::Ended);
        assert!(ended.ended_at_ms.is_some());
        meeting.id
    };

    let store = MeetingStore::open(&db_path).expect("reopen db");
    let loaded = store.get_meeting(&meeting_id, true).expect("get meeting");
    assert_eq!(loaded.status, MeetingStatus::Ended);
    assert_eq!(loaded.segment_count_outbound, Some(3));
    assert_eq!(loaded.segment_count_inbound, Some(2));

    let hits = store
        .search_segments("budget", Some(&meeting_id), None, Some(10))
        .expect("fts after reopen");
    assert_eq!(hits.len(), 2);

    let list = store.list_meetings(None, 10, 0).expect("list meetings");
    assert_eq!(list.total, 1);
}

#[test]
fn meeting_store_opens_fresh_file_with_schema_init() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("nested").join("meetings.db");
    let store = MeetingStore::open(&db_path).expect("open fresh db");
    let meeting = store
        .create_meeting("Notes", None, "vi", "vi", "notes", MeetingStatus::Live)
        .expect("create meeting");
    assert_eq!(meeting.session_mode, "notes");
}
