export type MeetingStatus = "live" | "ended";

export interface MeetingFolder {
  id: string;
  name: string;
  parentId: string | null;
  sortOrder: number;
  createdAtMs: number;
}

export interface MeetingRecord {
  id: string;
  folderId: string | null;
  title: string;
  myLanguage: string;
  meetingLanguage: string;
  sessionMode?: "interpreter" | "notes";
  /** Wall-clock Unix ms — library dates + Live header meeting timer. */
  startedAtMs: number;
  endedAtMs: number | null;
  status: MeetingStatus;
  segmentCountOutbound?: number;
  segmentCountInbound?: number;
  hasSummary?: boolean;
}

export interface MeetingListResponse {
  meetings: MeetingRecord[];
  total: number;
}

export interface TranscriptSegment {
  id: string;
  meetingId: string;
  direction: "outbound" | "inbound";
  sequence: number;
  sourceText: string;
  translatedText: string;
  /** Meeting-relative ms (0 = meeting start). Syncs with header meeting timer. */
  startedAtMs: number;
  /** Meeting-relative ms when the segment was committed. */
  endedAtMs: number;
  connectionGap: boolean;
}

export interface SegmentListResponse {
  segments: TranscriptSegment[];
  total: number;
  hasMoreOlder: boolean;
}

export interface SegmentCommittedEvent {
  meetingId: string;
  segment: TranscriptSegment;
}

export interface SummaryProgressEvent {
  meetingId: string;
  phase: string;
  current: number;
  total: number;
  /** Optional partial summary text from incremental merge. */
  partialText?: string;
}

export interface SummaryDoneEvent {
  meetingId: string;
}

export interface SummaryErrorEvent {
  meetingId: string;
  message: string;
}

/** Restore view of an in-flight summary generation. */
export interface SummaryGenerationStatus {
  meetingId: string;
  phase: string;
  current: number;
  total: number;
  startedAtMs: number;
}

/** Short button label + full detail for hover tooltip. */
export interface SummaryProgressUi {
  label: string;
  detail: string;
  partialText?: string;
}

export type SegmentPreviewReason = "sentence" | "turn" | "gap";

export interface SegmentPreviewEvent {
  meetingId: string;
  direction: "outbound" | "inbound";
  sequence: number;
  sourceText: string;
  translatedText: string;
  connectionGap: boolean;
  reason: SegmentPreviewReason;
}

export interface SegmentNeighbors {
  prev: TranscriptSegment | null;
  current: TranscriptSegment;
  next: TranscriptSegment | null;
}

export interface SegmentSearchHit {
  meetingId: string;
  segmentId: string;
  snippet: string;
  rank: number;
  startedAtMs?: number;
  direction?: "outbound" | "inbound";
}

export interface MeetingSearchHit {
  meetingId: string;
  title: string;
  snippet: string;
  rank: number;
  bestSegmentId: string;
  /** Meeting-relative ms of the best matching segment (0 = meeting start). */
  startedAtMs?: number;
}

/** Jump target from Library search → Detail timeline. */
export interface PendingScrollSegment {
  meetingId: string;
  segmentId: string;
  direction?: "outbound" | "inbound";
}

export interface MeetingSummary {
  meetingId: string;
  templateId: string;
  summaryLanguage: string;
  /** TipTap document JSON (SSOT for review + edit). */
  generatedJson: string;
  /** Derived export prose (computed from TipTap on read). */
  generatedText: string;
  generatedAtMs: number | null;
}

export interface SegmentCitation {
  segmentId: string;
  direction?: string;
  startedAtMs?: number;
  /** Transcript-segment citation metadata on summary/artifact rows. */
  meetingId?: string;
}

export type ArtifactKind = "decision" | "action_item" | "open_question";
export type ArtifactStatus = "proposed" | "confirmed" | "dismissed";
/** Provenance for curation — extracted wiped on regenerate. */
export type ArtifactOrigin = "extracted" | "edited" | "manual";

export interface MeetingArtifact {
  id: string;
  kind: string;
  text: string;
  owner?: string;
  due?: string;
  status: string;
  citations: SegmentCitation[];
  /** Absent on older payloads = treat as extracted. */
  origin?: ArtifactOrigin;
}

export interface MeetingEntity {
  id: string;
  name: string;
  kind: string;
  /** Absent on older payloads = treat as extracted. */
  origin?: ArtifactOrigin;
}

export interface MeetingChangedEvent {
  activeMeetingId: string | null;
  title: string | null;
  status: MeetingStatus | null;
}

export const UNCATEGORIZED_FOLDER_ID = "__uncategorized__";

export function formatMeetingDate(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  const date = d.toLocaleDateString([], {
    month: "short",
    day: "numeric",
    year: d.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
  });
  const time = d.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  return `${date}, ${time}`;
}

export function formatLanguagePair(my: string, meeting: string): string {
  return `${my.toUpperCase()} → ${meeting.toUpperCase()}`;
}
