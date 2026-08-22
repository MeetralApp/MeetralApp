//! Shared persist path for SegmentEngine commits (hot-path enqueue + sync flush).

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::meeting::segment_engine::{CommitReason, SegmentCommit};
use crate::meeting::transcript_writer::SegmentPersistJob;
use crate::meeting::writer_task::TranscriptDbWriterHandle;
use crate::meeting::{
    touch_meeting_idle_clock, MeetingIdleClock, MeetingStore, SegmentPreviewEvent,
    SharedTranscriptWriter,
};

pub fn commit_reason_label(reason: CommitReason) -> &'static str {
    match reason {
        CommitReason::Sentence => "sentence",
        CommitReason::Turn => "turn",
        CommitReason::Gap => "gap",
        CommitReason::Flush => "flush",
    }
}

fn emit_preview(app: &AppHandle, meeting_id: &str, commit: &SegmentCommit) {
    let preview = SegmentPreviewEvent {
        meeting_id: meeting_id.to_string(),
        direction: commit.direction.clone(),
        sequence: commit.sequence,
        source_text: commit.source.clone(),
        translated_text: commit.translated.clone(),
        connection_gap: commit.connection_gap,
        reason: commit_reason_label(commit.reason).to_string(),
    };
    let _ = app.emit("segment-preview", &preview);
}

fn to_persist_job(meeting_id: &str, commit: SegmentCommit) -> SegmentPersistJob {
    SegmentPersistJob {
        meeting_id: meeting_id.to_string(),
        direction: commit.direction,
        sequence: commit.sequence,
        source_text: commit.source,
        translated_text: commit.translated,
        connection_gap: commit.connection_gap,
        opened_at_mono: commit.opened_at_mono,
    }
}

fn touch_idle_if_needed(app: &AppHandle, touched: bool) {
    if !touched {
        return;
    }
    if let Some(clock) = app.try_state::<MeetingIdleClock>() {
        touch_meeting_idle_clock(clock.inner());
    }
}

/// Hot path: enqueue commits onto the async DB writer (non-blocking).
pub fn enqueue_segment_commits(
    app: &AppHandle,
    meeting_id: &str,
    commits: Vec<SegmentCommit>,
    db_writer: &TranscriptDbWriterHandle,
) {
    if commits.is_empty() {
        return;
    }
    let mut touched = false;
    for commit in commits {
        emit_preview(app, meeting_id, &commit);
        db_writer.enqueue(to_persist_job(meeting_id, commit));
        touched = true;
    }
    touch_idle_if_needed(app, touched);
}

/// Stop/end path: persist commits synchronously so Detail sees them immediately.
pub fn persist_segment_commits_sync(
    app: &AppHandle,
    meeting_id: &str,
    commits: Vec<SegmentCommit>,
) {
    if commits.is_empty() {
        return;
    }
    let store = app.state::<Arc<MeetingStore>>().inner().clone();
    let writer = app.state::<Arc<SharedTranscriptWriter>>().inner().clone();
    let mut touched = false;
    for commit in commits {
        let direction = commit.direction.clone();
        let sequence = commit.sequence;
        emit_preview(app, meeting_id, &commit);
        let job = to_persist_job(meeting_id, commit);
        if writer.persist(&job, &store, app).is_none() {
            tracing::error!(
                meeting_id = %meeting_id,
                direction = %direction,
                sequence,
                "sync flush persist failed; preview emitted but segment may be missing from store"
            );
        }
        touched = true;
    }
    touch_idle_if_needed(app, touched);
}
