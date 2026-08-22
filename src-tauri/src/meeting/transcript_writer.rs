//! Async persistence for segment commits allocated by SegmentEngine on the hot path.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter};

use crate::audio::monotonic_ms;
use crate::meeting::models::{SegmentCommittedEvent, TranscriptSegmentView};
use crate::meeting::store::MeetingStore;
use crate::meeting::time::{capture_mono_anchor, relative_ms};

#[derive(Debug, Clone)]
pub struct SegmentPersistJob {
    pub meeting_id: String,
    pub direction: String,
    pub sequence: i32,
    pub source_text: String,
    pub translated_text: String,
    pub connection_gap: bool,
    /// Process mono ms when the live utterance opened; `None` → use commit time (gaps).
    pub opened_at_mono: Option<i64>,
}

/// Persists segments with **meeting-relative** `started_at_ms` / `ended_at_ms`.
pub struct SharedTranscriptWriter {
    /// Monotonic ms corresponding to meeting wall-start (see `meeting::time`).
    mono_anchor_ms: Mutex<i64>,
}

impl SharedTranscriptWriter {
    pub fn new(mono_anchor_ms: i64) -> Self {
        Self {
            mono_anchor_ms: Mutex::new(mono_anchor_ms),
        }
    }

    /// Recompute mono anchor from the meeting's wall `started_at_ms` (create or app restart).
    pub fn set_meeting_wall_started_ms(&self, meeting_wall_started_ms: i64) {
        let anchor = capture_mono_anchor(meeting_wall_started_ms);
        *crate::meeting::lock_poison_recover(&self.mono_anchor_ms, "transcript writer") = anchor;
    }

    pub fn persist(
        &self,
        job: &SegmentPersistJob,
        store: &MeetingStore,
        app: &AppHandle,
    ) -> Option<TranscriptSegmentView> {
        let mono_now = monotonic_ms() as i64;
        let anchor =
            *crate::meeting::lock_poison_recover(&self.mono_anchor_ms, "transcript writer");
        let ended = relative_ms(mono_now, anchor);
        let started = job
            .opened_at_mono
            .map(|opened| relative_ms(opened, anchor))
            .unwrap_or(ended)
            .min(ended);

        let segment = store
            .insert_segment_at_sequence(
                &job.meeting_id,
                &job.direction,
                job.sequence,
                &job.source_text,
                &job.translated_text,
                started,
                ended,
                job.connection_gap,
            )
            .map_err(|e| {
                tracing::error!(
                    meeting_id = %job.meeting_id,
                    direction = %job.direction,
                    sequence = job.sequence,
                    "failed to persist transcript segment: {e:#}"
                );
                e
            })
            .ok()?;

        emit_segment(app, &job.meeting_id, segment.clone());
        Some(segment)
    }
}

fn emit_segment(app: &AppHandle, meeting_id: &str, segment: TranscriptSegmentView) {
    let _ = app.emit(
        "segment-committed",
        SegmentCommittedEvent {
            meeting_id: meeting_id.to_string(),
            segment,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::time::mono_anchor_for_meeting;

    #[test]
    fn started_and_ended_are_both_relative() {
        let meeting_wall = 1_700_000_000_000;
        let wall_now = meeting_wall + 12_000;
        let mono_now = 80_000;
        let anchor = mono_anchor_for_meeting(meeting_wall, wall_now, mono_now);
        let opened = mono_now - 3_000;
        let started = relative_ms(opened, anchor);
        let ended = relative_ms(mono_now, anchor);
        assert_eq!(started, 9_000);
        assert_eq!(ended, 12_000);
        assert!(started <= ended);
    }
}
