use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingFolderView {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MeetingStatus {
    Live,
    Ended,
}

impl MeetingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Ended => "ended",
        }
    }
}

impl std::str::FromStr for MeetingStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "live" => Ok(Self::Live),
            "ended" => Ok(Self::Ended),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRecordView {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub my_language: String,
    pub meeting_language: String,
    /// `interpreter` | `notes` — snapshot at create.
    pub session_mode: String,
    /// Wall-clock Unix ms (library dates + header meeting timer).
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub status: MeetingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_count_outbound: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_count_inbound: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_summary: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingListResponse {
    pub meetings: Vec<MeetingRecordView>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegmentView {
    pub id: String,
    pub meeting_id: String,
    pub direction: String,
    pub sequence: i32,
    pub source_text: String,
    pub translated_text: String,
    /// Meeting-relative ms (0 = `meeting_record.started_at_ms` wall instant).
    pub started_at_ms: i64,
    /// Meeting-relative ms when the segment was committed.
    pub ended_at_ms: i64,
    pub connection_gap: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentListResponse {
    pub segments: Vec<TranscriptSegmentView>,
    pub total: i64,
    pub has_more_older: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentNeighborsView {
    pub prev: Option<TranscriptSegmentView>,
    pub current: TranscriptSegmentView,
    pub next: Option<TranscriptSegmentView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummaryView {
    pub meeting_id: String,
    pub template_id: String,
    pub summary_language: String,
    /// TipTap document JSON (SSOT for review + edit).
    pub generated_json: String,
    /// Derived export prose from TipTap (not stored — computed on read).
    pub generated_text: String,
    pub generated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingChangedEvent {
    pub active_meeting_id: Option<String>,
    pub title: Option<String>,
    pub status: Option<MeetingStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryProgressEvent {
    pub meeting_id: String,
    pub phase: String,
    pub current: u32,
    pub total: u32,
    /// Optional partial summary text (streaming incremental merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_text: Option<String>,
}

/// Emitted when a background-safe summary run finishes. Lets any
/// mounted detail view refetch without the user manually refreshing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryDoneEvent {
    pub meeting_id: String,
}

/// Emitted when a summary run fails — the error otherwise dies silently when
/// the invoking view is unmounted.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryErrorEvent {
    pub meeting_id: String,
    pub message: String,
}

/// FE restore view of an in-flight summary generation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryGenerationStatusView {
    pub meeting_id: String,
    pub phase: String,
    pub current: u32,
    pub total: u32,
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentPreviewEvent {
    pub meeting_id: String,
    pub direction: String,
    pub sequence: i32,
    pub source_text: String,
    pub translated_text: String,
    pub connection_gap: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentCommittedEvent {
    pub meeting_id: String,
    pub segment: TranscriptSegmentView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSearchHit {
    pub meeting_id: String,
    pub segment_id: String,
    pub snippet: String,
    pub rank: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSearchHit {
    pub meeting_id: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
    pub best_segment_id: String,
    /// Meeting-relative ms of the best matching segment (0 = meeting start).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
}

/// Transcript-segment citation on summaries and artifacts (timestamp chip).
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentCitation {
    pub segment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactView {
    pub id: String,
    pub kind: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    pub status: String,
    pub citations: Vec<SegmentCitation>,
    /// `extracted` (LLM) | `edited` (user content edit) | `manual` (user-added).
    pub origin: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityView {
    pub id: String,
    pub name: String,
    pub kind: String,
    /// `extracted` (LLM) | `edited` (user rename) | `manual` (user-added).
    pub origin: String,
}
