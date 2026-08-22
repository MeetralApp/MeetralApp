use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::config::AppConfig;
use crate::error;
use crate::meeting::models::{
    ArtifactView, EntityView, MeetingFolderView, MeetingListResponse, MeetingRecordView,
    MeetingSearchHit, MeetingStatus, MeetingSummaryView, SegmentCitation, SegmentListResponse,
    SegmentNeighborsView, SegmentSearchHit,
};
use crate::meeting::prompts::{
    list_summary_languages_for_provider, list_summary_template_infos, DEFAULT_SUMMARY_TEMPLATE_ID,
};
use crate::meeting::store::{ArtifactRow, EntityRow};
use crate::meeting::summary_gen_registry::{
    StartError, SummaryGenerationRegistry, SUMMARY_ALREADY_RUNNING,
};
use crate::meeting::summary_service;
use crate::meeting::{self, meeting_db_path, ActiveMeetingId, MeetingStore};

async fn with_store<T, F>(store: Arc<MeetingStore>, f: F) -> Result<T, String>
where
    F: FnOnce(&MeetingStore) -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || f(&store))
        .await
        .map_err(|e| format!("database task failed: {e}"))?
        .map_err(error::log_and_stringify)
}

#[tauri::command]
pub async fn list_meeting_folders(
    store: State<'_, Arc<MeetingStore>>,
) -> Result<Vec<MeetingFolderView>, String> {
    with_store(store.inner().clone(), |s| s.list_folders()).await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<String>,
}

#[tauri::command]
pub async fn create_meeting_folder(
    request: CreateFolderRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingFolderView, String> {
    with_store(store.inner().clone(), move |s| {
        s.create_folder(&request.name, request.parent_id.as_deref())
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameFolderRequest {
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub async fn rename_meeting_folder(
    request: RenameFolderRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingFolderView, String> {
    with_store(store.inner().clone(), move |s| {
        s.rename_folder(&request.id, &request.name)
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFolderRequest {
    pub id: String,
}

#[tauri::command]
pub async fn delete_meeting_folder(
    request: DeleteFolderRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<(), String> {
    with_store(store.inner().clone(), move |s| {
        s.delete_folder(&request.id).map(|_| ())
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderFoldersRequest {
    pub folder_ids: Vec<String>,
}

#[tauri::command]
pub async fn reorder_meeting_folders(
    request: ReorderFoldersRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<Vec<MeetingFolderView>, String> {
    with_store(store.inner().clone(), move |s| {
        s.reorder_folders(&request.folder_ids)
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMeetingRequest {
    pub title: Option<String>,
    pub folder_id: Option<String>,
}

#[tauri::command]
pub async fn create_meeting(
    request: CreateMeetingRequest,
    app: AppHandle,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
    active: State<'_, ActiveMeetingId>,
) -> Result<MeetingRecordView, String> {
    let config = config.lock().await.clone();
    let record_audio = config.record_meeting_audio;
    let audio_folder = config.meeting_audio_save_folder.clone();
    let title = request.title.unwrap_or_else(|| {
        meeting::format_meeting_title(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        )
    });
    let folder_id = request.folder_id.clone();
    let meeting = with_store(store.inner().clone(), move |s| {
        s.create_meeting(
            &title,
            folder_id.as_deref(),
            &config.my_language,
            &config.meeting_language,
            if config.session_mode.is_notes() {
                "notes"
            } else {
                "interpreter"
            },
            MeetingStatus::Live,
        )
    })
    .await?;
    {
        let mut guard = crate::meeting::lock_poison_recover(active.inner(), "active meeting");
        *guard = Some(meeting.id.clone());
    }
    meeting::emit_meeting_changed(
        &app,
        Some(meeting.id.clone()),
        Some(meeting.title.clone()),
        Some(meeting.status.clone()),
    );
    meeting::emit_meeting_updated(&app, &meeting);
    if let Some(clock) = app.try_state::<meeting::MeetingIdleClock>() {
        meeting::touch_meeting_idle_clock(clock.inner());
    }
    if let Err(e) = meeting::maybe_start_for_meeting(
        store.inner().clone(),
        &app,
        &meeting.id,
        record_audio,
        &audio_folder,
    ) {
        tracing::warn!(error = %e, "failed to start meeting audio recording");
    }
    Ok(meeting)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingIdRequest {
    pub id: String,
}

#[tauri::command]
pub async fn end_meeting(
    request: MeetingIdRequest,
    app: AppHandle,
    store: State<'_, Arc<MeetingStore>>,
    active: State<'_, ActiveMeetingId>,
) -> Result<MeetingRecordView, String> {
    let id = request.id.clone();
    let is_active = {
        let guard = crate::meeting::lock_poison_recover(active.inner(), "active meeting");
        guard.as_deref() == Some(id.as_str())
    };
    if is_active {
        // Flush live transcript while active id still allows persist.
        meeting::finalize_meeting_transcripts(&app, &id, &["outbound", "inbound"]);
        meeting::flush_active_recording();
    } else {
        tracing::warn!(
            meeting_id = %id,
            "end_meeting called for non-active meeting; skipping transcript flush"
        );
    }

    let ended = with_store(store.inner().clone(), move |s| s.end_meeting(&id)).await?;
    let cleared_active = {
        let mut guard = crate::meeting::lock_poison_recover(active.inner(), "active meeting");
        if guard.as_deref() == Some(request.id.as_str()) {
            *guard = None;
            true
        } else {
            false
        }
    };
    if cleared_active {
        if let Some(clock) = app.try_state::<meeting::MeetingIdleClock>() {
            meeting::clear_meeting_idle_clock(clock.inner());
        }
    }
    meeting::emit_meeting_changed(
        &app,
        None,
        Some(ended.title.clone()),
        Some(MeetingStatus::Ended),
    );
    meeting::emit_meeting_updated(&app, &ended);
    Ok(ended)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameMeetingRequest {
    pub id: String,
    pub title: String,
}

#[tauri::command]
pub async fn rename_meeting(
    request: RenameMeetingRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingRecordView, String> {
    with_store(store.inner().clone(), move |s| {
        s.rename_meeting(&request.id, &request.title)
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveMeetingRequest {
    pub id: String,
    pub folder_id: Option<String>,
}

#[tauri::command]
pub async fn move_meeting(
    request: MoveMeetingRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingRecordView, String> {
    with_store(store.inner().clone(), move |s| {
        s.move_meeting(&request.id, request.folder_id.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn delete_meeting(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
    active: State<'_, ActiveMeetingId>,
) -> Result<(), String> {
    let id = request.id.clone();
    let is_active = {
        let guard = crate::meeting::lock_poison_recover(active.inner(), "active meeting");
        guard.as_deref() == Some(id.as_str())
    };
    if is_active {
        meeting::flush_active_recording();
    }
    // Collect recording dir from chunk paths before CASCADE delete.
    let paths = with_store(store.inner().clone(), {
        let id = id.clone();
        move |s| s.list_audio_chunk_paths(&id)
    })
    .await
    .unwrap_or_default();
    with_store(store.inner().clone(), move |s| {
        s.delete_meeting(&id).map(|_| ())
    })
    .await?;
    {
        let mut guard = crate::meeting::lock_poison_recover(active.inner(), "active meeting");
        if guard.as_deref() == Some(request.id.as_str()) {
            *guard = None;
        }
    }
    delete_meeting_audio_dirs(&paths);
    Ok(())
}

fn delete_meeting_audio_dirs(paths: &[String]) {
    use std::collections::HashSet;
    use std::path::PathBuf;
    let mut dirs = HashSet::new();
    for p in paths {
        let path = PathBuf::from(p);
        // .../meeting_id/direction/seq.opus → meeting_id dir
        if let Some(meeting_dir) = path.parent().and_then(|d| d.parent()) {
            dirs.insert(meeting_dir.to_path_buf());
        }
    }
    for dir in dirs {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            tracing::warn!(path = %dir.display(), error = %e, "failed to remove meeting recordings dir");
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMeetingsRequest {
    pub folder_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[tauri::command]
pub async fn list_meetings(
    request: ListMeetingsRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingListResponse, String> {
    let folder_id = request.folder_id.clone();
    let limit = request.limit.unwrap_or(50);
    let offset = request.offset.unwrap_or(0);
    with_store(store.inner().clone(), move |s| {
        s.list_meetings(folder_id.as_deref(), limit, offset)
    })
    .await
}

#[tauri::command]
pub async fn get_meeting(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingRecordView, String> {
    let id = request.id.clone();
    with_store(store.inner().clone(), move |s| s.get_meeting(&id, true)).await
}

#[tauri::command]
pub async fn get_active_meeting(
    store: State<'_, Arc<MeetingStore>>,
    active: State<'_, ActiveMeetingId>,
) -> Result<Option<MeetingRecordView>, String> {
    let id = {
        let guard = crate::meeting::lock_poison_recover(active.inner(), "active meeting");
        guard.clone()
    };
    if let Some(id) = id {
        return with_store(store.inner().clone(), move |s| {
            s.get_meeting(&id, true).map(Some)
        })
        .await;
    }
    with_store(store.inner().clone(), |s| s.get_live_meeting()).await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSegmentsRequest {
    pub meeting_id: String,
    pub direction: Option<String>,
    pub from_sequence: Option<i32>,
    pub before_sequence: Option<i32>,
    pub tail: Option<bool>,
    pub limit: Option<i64>,
}

#[tauri::command]
pub async fn list_meeting_segments(
    request: ListSegmentsRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<SegmentListResponse, String> {
    let meeting_id = request.meeting_id.clone();
    let direction = request.direction.clone();
    let from_sequence = request.from_sequence;
    let before_sequence = request.before_sequence;
    let tail = request.tail.unwrap_or(false);
    let limit = request.limit.unwrap_or(100);
    with_store(store.inner().clone(), move |s| {
        s.list_segments(
            &meeting_id,
            direction.as_deref(),
            from_sequence,
            before_sequence,
            tail,
            limit,
        )
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSegmentsRequest {
    pub query: String,
    pub meeting_id: Option<String>,
    pub folder_id: Option<String>,
    pub limit: Option<u32>,
}

#[tauri::command]
pub async fn search_segments(
    request: SearchSegmentsRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<Vec<SegmentSearchHit>, String> {
    let query = request.query.clone();
    let meeting_id = request.meeting_id.clone();
    let folder_id = request.folder_id.clone();
    let limit = request.limit.map(|n| n as usize);
    with_store(store.inner().clone(), move |s| {
        s.search_segments(&query, meeting_id.as_deref(), folder_id.as_deref(), limit)
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMeetingsRequest {
    pub query: String,
    pub folder_id: Option<String>,
    pub limit: Option<u32>,
}

#[tauri::command]
pub async fn search_meetings(
    request: SearchMeetingsRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<Vec<MeetingSearchHit>, String> {
    let query = request.query.clone();
    let folder_id = request.folder_id.clone();
    let limit = request.limit.map(|n| n as usize);
    with_store(store.inner().clone(), move |s| {
        s.search_meetings(&query, folder_id.as_deref(), limit)
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentNeighborsRequest {
    pub meeting_id: String,
    pub segment_id: String,
}

#[tauri::command]
pub async fn get_segment_neighbors(
    request: SegmentNeighborsRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<SegmentNeighborsView, String> {
    let meeting_id = request.meeting_id.clone();
    let segment_id = request.segment_id.clone();
    with_store(store.inner().clone(), move |s| {
        s.get_segment_neighbors(&meeting_id, &segment_id)
    })
    .await
}

#[tauri::command]
pub async fn get_meeting_summary(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<Option<MeetingSummaryView>, String> {
    let id = request.id.clone();
    with_store(store.inner().clone(), move |s| s.get_summary(&id)).await
}

#[tauri::command]
pub fn list_summary_templates_cmd() -> Vec<crate::meeting::prompts::SummaryTemplateInfo> {
    list_summary_template_infos()
}

#[tauri::command]
pub fn list_summary_languages_cmd(
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<crate::meeting::prompts::SummaryLanguageInfo>, String> {
    let guard = config.blocking_lock();
    Ok(list_summary_languages_for_provider(guard.ai_provider))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSummaryRequest {
    pub id: String,
    #[serde(default = "default_summary_template_id")]
    pub template_id: String,
    pub language: String,
}

fn default_summary_template_id() -> String {
    DEFAULT_SUMMARY_TEMPLATE_ID.to_string()
}

#[tauri::command]
pub async fn generate_meeting_summary(
    request: GenerateSummaryRequest,
    app: AppHandle,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
    gen_registry: State<'_, SummaryGenerationRegistry>,
) -> Result<MeetingSummaryView, String> {
    let (selection, meeting_context) = {
        let guard = config.lock().await;
        match guard.summary_llm_selection() {
            Some(selection) => (selection, guard.meeting_context.clone()),
            None => {
                return Err("Add a Gemini or OpenAI API key to generate meeting summaries.".into());
            }
        }
    };
    let summary_model = selection.chat_model().to_string();
    let client = crate::runtime::factories::summary_llm_client_for_selection(&selection)
        .map_err(error::log_and_stringify)?;
    let meeting_id = request.id.clone();
    // Double-run guard: a second generate for the same meeting is
    // rejected instead of racing a duplicate LLM job. Guard drop clears the
    // registry entry on every exit path below.
    let _gen_guard = gen_registry
        .try_start(&meeting_id, crate::meeting::store::wall_ms())
        .map_err(|err| match err {
            StartError::AlreadyRunning => SUMMARY_ALREADY_RUNNING.to_string(),
        })?;
    crate::debug_log!(
        meeting_id = %meeting_id,
        provider = %selection.origin_label(),
        model = %summary_model,
        template_id = %request.template_id,
        language = %request.language,
        "cmd generate_meeting_summary"
    );
    summary_service::generate_meeting_summary(
        store.inner(),
        &meeting_id,
        &client,
        &summary_model,
        &request.template_id,
        &request.language,
        &meeting_context,
        Some(&app),
        Some(gen_registry.inner()),
    )
    .await
    .map_err(|err| {
        // Surface failures to any mounted view — the invoking view may already
        // be unmounted, in which case its promise rejection is lost.
        let message = error::log_and_stringify(err);
        let _ = app.emit(
            "summary-error",
            meeting::models::SummaryErrorEvent {
                meeting_id: meeting_id.clone(),
                message: message.clone(),
            },
        );
        message
    })?;
    let _ = app.emit(
        "summary-done",
        meeting::models::SummaryDoneEvent {
            meeting_id: meeting_id.clone(),
        },
    );
    with_store(store.inner().clone(), move |s| {
        s.get_summary(&meeting_id)?
            .ok_or_else(|| anyhow::anyhow!("summary missing after generate"))
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryGenerationStatusRequest {
    pub id: String,
}

/// FE restore after remount: is a summary generation in flight for this
/// meeting, and at which phase.
#[tauri::command]
pub async fn get_summary_generation_status(
    request: SummaryGenerationStatusRequest,
    gen_registry: State<'_, SummaryGenerationRegistry>,
) -> Result<Option<meeting::models::SummaryGenerationStatusView>, String> {
    Ok(gen_registry
        .status(&request.id)
        .map(|state| meeting::models::SummaryGenerationStatusView {
            meeting_id: request.id.clone(),
            phase: state.phase,
            current: state.current,
            total: state.total,
            started_at_ms: state.started_at_ms,
        }))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSummaryRequest {
    pub meeting_id: String,
    /// TipTap document JSON — overwrites `generated_json`.
    pub doc_json: String,
}

#[tauri::command]
pub async fn update_meeting_summary(
    request: UpdateSummaryRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingSummaryView, String> {
    let meeting_id = request.meeting_id.clone();
    let doc_json = request.doc_json.clone();
    with_store(store.inner().clone(), move |s| {
        s.update_summary_doc(&meeting_id, &doc_json)
    })
    .await
}

pub fn init_meeting_store(app: &AppHandle) -> Result<MeetingStore, String> {
    let path = meeting_db_path(app).map_err(error::log_and_stringify)?;
    MeetingStore::open(&path).map_err(error::log_and_stringify)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListArtifactsRequest {
    pub meeting_id: String,
    pub kind: Option<String>,
    pub status: Option<String>,
}

#[tauri::command]
pub async fn list_meeting_artifacts(
    request: ListArtifactsRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<meeting::models::ArtifactView>, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        let rows = s.list_artifacts(
            &request.meeting_id,
            request.kind.as_deref(),
            request.status.as_deref(),
        )?;
        let mut views = Vec::with_capacity(rows.len());
        for row in rows {
            let citations = s.resolve_segment_refs(&row.meeting_id, &row.segment_refs)?;
            views.push(artifact_view(row, citations));
        }
        Ok(views)
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetArtifactStatusRequest {
    pub id: String,
    pub status: String,
}

#[tauri::command]
pub async fn set_artifact_status(
    request: SetArtifactStatusRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<meeting::models::ArtifactView, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        let row = s
            .set_artifact_status(&request.id, &request.status)?
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {}", request.id))?;
        let citations = s.resolve_segment_refs(&row.meeting_id, &row.segment_refs)?;
        Ok(artifact_view(row, citations))
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArtifactRequest {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
}

/// Edit an artifact's text/owner/due. Re-keys extracted rows into the
/// user namespace so a regenerate never collides. Returns the updated view.
#[tauri::command]
pub async fn update_meeting_artifact(
    request: UpdateArtifactRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<meeting::models::ArtifactView, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        let row = s
            .update_artifact_content(
                &request.id,
                &request.text,
                request.owner.as_deref(),
                request.due.as_deref(),
            )?
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {}", request.id))?;
        let citations = s.resolve_segment_refs(&row.meeting_id, &row.segment_refs)?;
        Ok(artifact_view(row, citations))
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArtifactRequest {
    pub meeting_id: String,
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
}

/// Manually add an artifact: origin 'manual', no citations.
#[tauri::command]
pub async fn create_meeting_artifact(
    request: CreateArtifactRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<meeting::models::ArtifactView, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        let row = s.create_artifact(
            &request.meeting_id,
            &request.kind,
            &request.text,
            request.owner.as_deref(),
            request.due.as_deref(),
            crate::meeting::store::wall_ms(),
        )?;
        let citations = s.resolve_segment_refs(&row.meeting_id, &row.segment_refs)?;
        Ok(artifact_view(row, citations))
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteArtifactRequest {
    pub id: String,
}

/// Hard-delete an artifact. False when the id does not exist.
#[tauri::command]
pub async fn delete_meeting_artifact(
    request: DeleteArtifactRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<bool, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        s.delete_artifact(&request.id)
    })
    .await
}

#[tauri::command]
pub async fn list_meeting_entities(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<Vec<meeting::models::EntityView>, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        Ok(s.list_entities(&request.id)?
            .into_iter()
            .map(entity_view)
            .collect())
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntityRequest {
    pub id: String,
    pub name: String,
    pub kind: String,
}

/// Rename / re-kind an entity. Keeps the name-keyed id stable;
/// updates entity_fts in the same transaction.
#[tauri::command]
pub async fn update_meeting_entity(
    request: UpdateEntityRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<meeting::models::EntityView, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        let row = s
            .update_entity_content(&request.id, &request.name, &request.kind)?
            .ok_or_else(|| anyhow::anyhow!("entity not found: {}", request.id))?;
        Ok(entity_view(row))
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntityRequest {
    pub meeting_id: String,
    pub name: String,
    pub kind: String,
}

/// Manually add a mentioned entity: origin 'manual'. Errors on an
/// existing exact name.
#[tauri::command]
pub async fn create_meeting_entity(
    request: CreateEntityRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<meeting::models::EntityView, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| {
        let row = s
            .create_entity(
                &request.meeting_id,
                &request.name,
                &request.kind,
                crate::meeting::store::wall_ms(),
            )?
            .ok_or_else(|| anyhow::anyhow!("entity already exists: {}", request.name))?;
        Ok(entity_view(row))
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEntityRequest {
    pub id: String,
}

/// Hard-delete a mentioned entity. False when the id does not exist.
#[tauri::command]
pub async fn delete_meeting_entity(
    request: DeleteEntityRequest,
    store: State<'_, Arc<MeetingStore>>,
    config: State<'_, AsyncMutex<AppConfig>>,
) -> Result<bool, String> {
    {
        let guard = config.lock().await;
        guard.ensure_artifacts_enabled()?;
    }
    with_store(store.inner().clone(), move |s| s.delete_entity(&request.id)).await
}

fn artifact_view(row: ArtifactRow, citations: Vec<SegmentCitation>) -> ArtifactView {
    ArtifactView {
        id: row.id,
        kind: row.kind,
        text: row.text,
        owner: row.owner,
        due: row.due,
        status: row.status,
        citations,
        origin: row.origin,
    }
}

fn entity_view(row: EntityRow) -> EntityView {
    EntityView {
        id: row.id,
        name: row.name,
        kind: row.kind,
        origin: row.origin,
    }
}

#[tauri::command]
pub async fn default_meeting_audio_folder(app: AppHandle) -> Result<String, String> {
    meeting::store::default_recordings_dir(&app)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(error::log_and_stringify)
}

#[tauri::command]
pub async fn pick_meeting_audio_folder() -> Result<Option<String>, String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Meeting audio folder")
        .pick_folder()
        .await;
    Ok(folder.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn open_meeting_audio_folder(
    app: AppHandle,
    config: State<'_, AsyncMutex<AppConfig>>,
    folder: Option<String>,
) -> Result<(), String> {
    let configured = match folder {
        Some(f) if !f.trim().is_empty() => f,
        _ => {
            let cfg = config.lock().await;
            cfg.meeting_audio_save_folder.clone()
        }
    };
    let path = meeting::store::resolve_recordings_base(&app, &configured)
        .map_err(error::log_and_stringify)?;
    std::fs::create_dir_all(&path).map_err(error::log_and_stringify)?;
    open_path_in_os(&path).map_err(error::log_and_stringify)
}

fn open_path_in_os(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_meeting_audio_chunks(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<Vec<meeting::store::AudioChunkRow>, String> {
    let id = request.id;
    with_store(store.inner().clone(), move |s| s.list_audio_chunks(&id)).await
}

#[tauri::command]
pub async fn meeting_audio_has_any(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<bool, String> {
    let id = request.id;
    with_store(store.inner().clone(), move |s| s.has_audio_chunks(&id)).await
}

/// Verify chunk files exist for a meeting; zero `byte_size` for missing paths.
/// Returns only playable chunks (files still on disk).
#[tauri::command]
pub async fn verify_meeting_audio(
    request: MeetingIdRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<meeting::store::VerifyMeetingAudioResult, String> {
    let id = request.id;
    with_store(store.inner().clone(), move |s| {
        s.verify_meeting_audio_chunks(&id)
    })
    .await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAudioDiskUsage {
    pub total_bytes: u64,
}

#[tauri::command]
pub async fn meeting_audio_disk_usage(
    store: State<'_, Arc<MeetingStore>>,
) -> Result<MeetingAudioDiskUsage, String> {
    with_store(store.inner().clone(), move |s| {
        Ok(MeetingAudioDiskUsage {
            total_bytes: s.total_audio_byte_size()?,
        })
    })
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodeAudioWindowRequest {
    pub meeting_id: String,
    /// `room` | `you` | `meeting`
    pub source: String,
    pub start_ms: i64,
    pub duration_ms: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodeAudioWindowResponse {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

#[tauri::command]
pub async fn decode_meeting_audio_window(
    request: DecodeAudioWindowRequest,
    store: State<'_, Arc<MeetingStore>>,
) -> Result<DecodeAudioWindowResponse, String> {
    let meeting_id = request.meeting_id.clone();
    let source = request.source.clone();
    let start_ms = request.start_ms.max(0);
    let duration_ms = request.duration_ms.clamp(100, 5_000);
    let chunks = with_store(store.inner().clone(), move |s| {
        s.list_audio_chunks(&meeting_id)
    })
    .await?;

    tokio::task::spawn_blocking(move || {
        meeting::recording::decode_window_sync(&chunks, &source, start_ms, duration_ms).map(
            |decoded| DecodeAudioWindowResponse {
                sample_rate: decoded.sample_rate,
                samples: decoded.samples,
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(error::log_and_stringify)
}
