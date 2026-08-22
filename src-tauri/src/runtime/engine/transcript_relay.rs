use std::sync::Arc;

use super::TranslationEngine;
use crate::ai::TranscriptEvent;
use crate::meeting::transcript_flush::{enqueue_segment_commits, persist_segment_commits_sync};
use crate::meeting::{ActiveMeetingId, MeetingStore, SharedSegmentEngine, SharedTranscriptWriter};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

impl TranslationEngine {
    pub(super) fn ensure_transcript_relay(
        &mut self,
        app: &AppHandle,
    ) -> mpsc::Sender<TranscriptEvent> {
        if let Some(tx) = &self.transcript_tx {
            return tx.clone();
        }

        if self.transcript_db_writer.is_none() {
            let store = app.state::<Arc<MeetingStore>>().inner().clone();
            let writer = app.state::<Arc<SharedTranscriptWriter>>().inner().clone();
            let metrics = self.transcript_db_metrics.clone();
            let (handle, _task) = crate::meeting::writer_task::spawn_transcript_db_writer(
                store,
                writer,
                app.clone(),
                metrics,
            );
            self.transcript_db_writer = Some(handle);
        }

        // Bounded: a stalled FE/db-writer drops newest live events
        // (counted via try_send_control at the fanout sites) instead of
        // growing memory; committed segments persist via the segment engine.
        let (tx, mut rx) = mpsc::channel::<TranscriptEvent>(
            crate::runtime::control_channel::TRANSCRIPT_FANOUT_CHANNEL_DEPTH,
        );
        let app_bg = app.clone();
        let db_writer = self.transcript_db_writer.clone().expect("db writer");
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                relay_transcript_event(&app_bg, &db_writer, event);
            }
        });
        self.transcript_tx = Some(tx.clone());
        tx
    }

    pub(super) fn clear_transcript_relay_if_idle(&mut self) {
        if !self.is_busy() {
            self.transcript_tx = None;
        }
    }

    /// Force-commit remaining live utterance text for the given directions, then seal
    /// those directions so late relay events cannot duplicate the flushed segments.
    /// Call **before** cancelling / aborting the provider bridge.
    pub fn flush_live_transcript_segments(&mut self, app: &AppHandle, directions: &[&str]) {
        let active = app.state::<ActiveMeetingId>().inner().clone();
        let meeting_id = match active.lock().ok().and_then(|g| g.clone()) {
            Some(id) => id,
            None => return,
        };

        let store = app.state::<Arc<MeetingStore>>().inner().clone();
        let segment_engine = app.state::<Arc<SharedSegmentEngine>>().inner().clone();
        let writer = app.state::<Arc<SharedTranscriptWriter>>().inner().clone();
        let relay_ctx = app
            .state::<crate::meeting::SharedRelayMeetingContext>()
            .inner()
            .clone();

        relay_ctx.ensure_prepared(&store, &segment_engine, &writer, &meeting_id);

        let mut commits = Vec::new();
        for direction in directions {
            if let Some(commit) = segment_engine.flush_direction(direction) {
                commits.push(commit);
            }
        }
        persist_segment_commits_sync(app, &meeting_id, commits);
    }
}

fn relay_transcript_event(
    app: &AppHandle,
    db_writer: &crate::meeting::writer_task::TranscriptDbWriterHandle,
    mut event: TranscriptEvent,
) {
    let store = app.state::<Arc<MeetingStore>>().inner().clone();
    let active = app.state::<ActiveMeetingId>().inner().clone();
    let segment_engine = app.state::<Arc<SharedSegmentEngine>>().inner().clone();
    let writer = app.state::<Arc<SharedTranscriptWriter>>().inner().clone();
    let relay_ctx = app
        .state::<crate::meeting::SharedRelayMeetingContext>()
        .inner()
        .clone();

    let meeting_id = active.lock().ok().and_then(|guard| guard.clone());
    let persist = meeting_id.is_some();

    if let Some(ref meeting_id) = meeting_id {
        relay_ctx.ensure_prepared(&store, &segment_engine, &writer, meeting_id);
    } else {
        relay_ctx.clear(&segment_engine);
    }

    let mut result = segment_engine.process(&event, persist);
    event.live_source = result.live.source.take();
    event.live_translated = result.live.translated.take();

    let _ = app.emit("transcript", &event);

    let Some(meeting_id) = meeting_id else {
        return;
    };

    enqueue_segment_commits(app, &meeting_id, result.commits, db_writer);
}
