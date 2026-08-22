//! Caches per-meeting DB preparation for transcript relay — avoids repeated SQLite reads.

use std::sync::{Arc, Mutex};

use crate::meeting::{MeetingStore, SharedSegmentEngine, SharedTranscriptWriter};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RelayMeetingState {
    meeting_id: Option<String>,
}

pub struct RelayMeetingContext {
    inner: Mutex<RelayMeetingState>,
}

impl Default for RelayMeetingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RelayMeetingContext {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RelayMeetingState::default()),
        }
    }

    pub fn prepared_meeting_id(&self) -> Option<String> {
        crate::meeting::lock_poison_recover(&self.inner, "relay meeting context")
            .meeting_id
            .clone()
    }

    pub fn ensure_prepared(
        &self,
        store: &MeetingStore,
        segment_engine: &SharedSegmentEngine,
        writer: &SharedTranscriptWriter,
        meeting_id: &str,
    ) {
        let mut state = crate::meeting::lock_poison_recover(&self.inner, "relay meeting context");
        if state.meeting_id.as_deref() == Some(meeting_id) {
            return;
        }

        let meeting_wall_started_ms = store
            .get_meeting(meeting_id, false)
            .ok()
            .map(|m| m.started_at_ms)
            .unwrap_or(0);
        writer.set_meeting_wall_started_ms(meeting_wall_started_ms);

        let outbound_next = store
            .max_segment_sequence(meeting_id, "outbound")
            .unwrap_or(0)
            + 1;
        let inbound_next = store
            .max_segment_sequence(meeting_id, "inbound")
            .unwrap_or(0)
            + 1;
        segment_engine.prepare_meeting(
            meeting_id,
            meeting_wall_started_ms,
            outbound_next,
            inbound_next,
        );
        state.meeting_id = Some(meeting_id.to_string());
    }

    pub fn clear(&self, segment_engine: &SharedSegmentEngine) {
        let mut state = crate::meeting::lock_poison_recover(&self.inner, "relay meeting context");
        if state.meeting_id.is_none() {
            return;
        }
        segment_engine.clear_meeting();
        state.meeting_id = None;
    }
}

pub type SharedRelayMeetingContext = Arc<RelayMeetingContext>;

pub fn new_shared_relay_meeting_context() -> SharedRelayMeetingContext {
    Arc::new(RelayMeetingContext::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::models::MeetingStatus;
    use crate::meeting::segment_engine::SharedSegmentEngine;

    #[test]
    fn ensure_prepared_skips_second_call_for_same_meeting() {
        let store = MeetingStore::open_in_memory().expect("open store");
        let meeting = store
            .create_meeting("Test", None, "en", "vi", "interpreter", MeetingStatus::Live)
            .expect("create meeting");
        let engine = SharedSegmentEngine::new(0);
        let writer = SharedTranscriptWriter::new(0);
        let ctx = RelayMeetingContext::new();

        ctx.ensure_prepared(&store, &engine, &writer, &meeting.id);
        assert_eq!(
            ctx.prepared_meeting_id().as_deref(),
            Some(meeting.id.as_str())
        );

        ctx.ensure_prepared(&store, &engine, &writer, &meeting.id);
        assert_eq!(
            ctx.prepared_meeting_id().as_deref(),
            Some(meeting.id.as_str())
        );
    }

    #[test]
    fn clear_resets_prepared_meeting() {
        let store = MeetingStore::open_in_memory().expect("open store");
        let meeting = store
            .create_meeting("Test", None, "en", "vi", "interpreter", MeetingStatus::Live)
            .expect("create meeting");
        let engine = SharedSegmentEngine::new(0);
        let writer = SharedTranscriptWriter::new(0);
        let ctx = RelayMeetingContext::new();

        ctx.ensure_prepared(&store, &engine, &writer, &meeting.id);
        ctx.clear(&engine);
        assert_eq!(ctx.prepared_meeting_id(), None);
    }
}
