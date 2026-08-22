//! Meeting session helpers — start on New meeting or first Translate;
//! end on End meeting, New meeting, or quit.

pub mod models;
pub mod prompts;
pub mod recording;
pub mod relay_meeting_context;
pub mod segment_engine;
pub mod store;
pub mod summary_doc;
pub mod summary_gen_registry;
pub mod summary_schema;
pub mod summary_service;
pub mod time;
pub mod transcript_flush;
pub mod transcript_writer;
pub mod writer_task;

pub use models::*;
pub use recording::{
    flush_active_recording, maybe_start_for_meeting, new_shared_active_recording,
    set_active_recording, tap_pcm, ActiveRecording, SharedActiveRecording,
};
pub use relay_meeting_context::{
    new_shared_relay_meeting_context, RelayMeetingContext, SharedRelayMeetingContext,
};
pub use segment_engine::SharedSegmentEngine;
pub use store::{format_meeting_title, meeting_db_path, MeetingStore};
pub use summary_gen_registry::{
    StartError, SummaryGenGuard, SummaryGenState, SummaryGenerationRegistry,
    SUMMARY_ALREADY_RUNNING,
};
pub use transcript_writer::SharedTranscriptWriter;

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, MutexGuard,
};

use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};

/// Recover from a poisoned meeting-layer mutex without cascading panic.
pub(crate) fn lock_poison_recover<'a, T>(
    mutex: &'a Mutex<T>,
    label: &'static str,
) -> MutexGuard<'a, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        tracing::error!("{label} mutex poisoned; recovering via into_inner");
        poisoned.into_inner()
    })
}

use crate::config::AppConfig;

/// Shared active meeting id between engine relay and commands.
pub type ActiveMeetingId = Arc<Mutex<Option<String>>>;

/// Monotonic ms of last segment commit (or meeting activation). `0` = no active tracking.
pub type MeetingIdleClock = Arc<AtomicU64>;

pub fn new_active_meeting_id() -> ActiveMeetingId {
    Arc::new(Mutex::new(None))
}

pub fn new_meeting_idle_clock() -> MeetingIdleClock {
    Arc::new(AtomicU64::new(0))
}

pub fn touch_meeting_idle_clock(clock: &MeetingIdleClock) {
    // `0` means inactive — never store a zero timestamp.
    clock.store(crate::audio::monotonic_ms().max(1), Ordering::Relaxed);
}

pub fn clear_meeting_idle_clock(clock: &MeetingIdleClock) {
    clock.store(0, Ordering::Relaxed);
}

pub fn meeting_idle_elapsed_ms(clock: &MeetingIdleClock, now_ms: u64) -> Option<u64> {
    let last = clock.load(Ordering::Relaxed);
    if last == 0 {
        None
    } else {
        Some(now_ms.saturating_sub(last))
    }
}

fn touch_idle_clock_from_app(app: &AppHandle) {
    if let Some(clock) = app.try_state::<MeetingIdleClock>() {
        touch_meeting_idle_clock(clock.inner());
    }
}

fn clear_idle_clock_from_app(app: &AppHandle) {
    if let Some(clock) = app.try_state::<MeetingIdleClock>() {
        clear_meeting_idle_clock(clock.inner());
    }
}

fn meeting_store_arc(app: &AppHandle) -> Option<std::sync::Arc<MeetingStore>> {
    app.try_state::<std::sync::Arc<MeetingStore>>()
        .map(|s| s.inner().clone())
}

fn start_recording_for_meeting(app: &AppHandle, meeting_id: &str, config: &AppConfig) {
    let Some(store) = meeting_store_arc(app) else {
        tracing::warn!("meeting store not ready; skip audio recording start");
        return;
    };
    if let Err(e) = recording::maybe_start_for_meeting(
        store,
        app,
        meeting_id,
        config.record_meeting_audio,
        &config.meeting_audio_save_folder,
    ) {
        tracing::warn!(error = %e, "failed to start meeting audio recording");
    }
}

pub fn emit_meeting_updated(app: &AppHandle, meeting: &models::MeetingRecordView) {
    let _ = app.emit("meeting-updated", meeting);
}

pub fn emit_meeting_changed(
    app: &AppHandle,
    active_meeting_id: Option<String>,
    title: Option<String>,
    status: Option<models::MeetingStatus>,
) {
    let _ = app.emit(
        "meeting-changed",
        models::MeetingChangedEvent {
            active_meeting_id,
            title,
            status,
        },
    );
}

pub fn ensure_active_meeting(
    app: &AppHandle,
    store: &MeetingStore,
    active: &ActiveMeetingId,
    config: &AppConfig,
) -> Result<String> {
    {
        let guard = lock_poison_recover(active, "active meeting");
        if let Some(id) = guard.as_ref() {
            return Ok(id.clone());
        }
    }
    if let Some(live) = store.get_live_meeting()? {
        let mut guard = lock_poison_recover(active, "active meeting");
        *guard = Some(live.id.clone());
        emit_meeting_changed(
            app,
            Some(live.id.clone()),
            Some(live.title.clone()),
            Some(live.status.clone()),
        );
        touch_idle_clock_from_app(app);
        start_recording_for_meeting(app, &live.id, config);
        return Ok(live.id);
    }

    let title = format_meeting_title(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
    );
    let meeting = store.create_meeting(
        &title,
        None,
        &config.my_language,
        &config.meeting_language,
        if config.session_mode.is_notes() {
            "notes"
        } else {
            "interpreter"
        },
        models::MeetingStatus::Live,
    )?;
    {
        let mut guard = lock_poison_recover(active, "active meeting");
        *guard = Some(meeting.id.clone());
    }
    emit_meeting_changed(
        app,
        Some(meeting.id.clone()),
        Some(meeting.title.clone()),
        Some(meeting.status.clone()),
    );
    emit_meeting_updated(app, &meeting);
    touch_idle_clock_from_app(app);
    start_recording_for_meeting(app, &meeting.id, config);
    Ok(meeting.id)
}

pub fn end_active_meeting(
    app: &AppHandle,
    store: &MeetingStore,
    active: &ActiveMeetingId,
) -> Result<()> {
    let id = {
        let guard = lock_poison_recover(active, "active meeting");
        guard.clone()
    };
    let Some(id) = id else {
        return Ok(());
    };
    // Flush any remaining live text while active id is still set (persist = true).
    finalize_meeting_transcripts(app, &id, &["outbound", "inbound"]);
    recording::flush_active_recording();
    let ended = store.end_meeting(&id)?;
    {
        let mut guard = lock_poison_recover(active, "active meeting");
        *guard = None;
    }
    clear_idle_clock_from_app(app);
    emit_meeting_changed(
        app,
        None,
        Some(ended.title.clone()),
        Some(models::MeetingStatus::Ended),
    );
    emit_meeting_updated(app, &ended);
    Ok(())
}

/// Force-commit remaining SegmentEngine live buffers for an active meeting.
/// No-op when `meeting_id` is not the current active meeting.
pub fn finalize_meeting_transcripts(app: &AppHandle, meeting_id: &str, directions: &[&str]) {
    let active = match app.try_state::<ActiveMeetingId>() {
        Some(state) => state.inner().clone(),
        None => return,
    };
    let is_active = active
        .lock()
        .ok()
        .map(|g| g.as_deref() == Some(meeting_id))
        .unwrap_or(false);
    if !is_active {
        return;
    }

    let Some(store) = app.try_state::<std::sync::Arc<MeetingStore>>() else {
        return;
    };
    let Some(segment_engine) = app.try_state::<std::sync::Arc<SharedSegmentEngine>>() else {
        return;
    };
    let Some(writer) = app.try_state::<std::sync::Arc<SharedTranscriptWriter>>() else {
        return;
    };
    let Some(relay_ctx) = app.try_state::<SharedRelayMeetingContext>() else {
        return;
    };

    relay_ctx.ensure_prepared(
        store.inner(),
        segment_engine.inner(),
        writer.inner(),
        meeting_id,
    );

    let mut commits = Vec::new();
    for direction in directions {
        if let Some(commit) = segment_engine.flush_direction(direction) {
            commits.push(commit);
        }
    }
    transcript_flush::persist_segment_commits_sync(app, meeting_id, commits);
}
