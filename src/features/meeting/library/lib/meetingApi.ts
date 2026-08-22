import { invoke } from "@tauri-apps/api/core";

import type {
  ArtifactKind,
  ArtifactStatus,
  MeetingArtifact,
  MeetingEntity,
  MeetingFolder,
  MeetingListResponse,
  MeetingRecord,
  MeetingSearchHit,
  MeetingSummary,
  SegmentCommittedEvent,
  SegmentListResponse,
  SegmentNeighbors,
  SegmentSearchHit,
  SummaryGenerationStatus,
} from "./meetingTypes";
import type { SummaryLanguageInfo, SummaryTemplateInfo } from "@/features/meeting/detail/lib/summaryTypes";

export async function listMeetingFolders(): Promise<MeetingFolder[]> {
  return invoke("list_meeting_folders");
}

export async function createMeetingFolder(
  name: string,
  parentId?: string | null,
): Promise<MeetingFolder> {
  return invoke("create_meeting_folder", {
    request: { name, parentId: parentId ?? null },
  });
}

export async function renameMeetingFolder(
  id: string,
  name: string,
): Promise<MeetingFolder> {
  return invoke("rename_meeting_folder", { request: { id, name } });
}

export async function deleteMeetingFolder(id: string): Promise<void> {
  return invoke("delete_meeting_folder", { request: { id } });
}

export async function reorderMeetingFolders(
  folderIds: string[],
): Promise<MeetingFolder[]> {
  return invoke("reorder_meeting_folders", { request: { folderIds } });
}

export async function createMeeting(
  title?: string,
  folderId?: string | null,
): Promise<MeetingRecord> {
  return invoke("create_meeting", {
    request: { title: title ?? null, folderId: folderId ?? null },
  });
}

export async function endMeeting(id: string): Promise<MeetingRecord> {
  return invoke("end_meeting", { request: { id } });
}

export async function renameMeeting(
  id: string,
  title: string,
): Promise<MeetingRecord> {
  return invoke("rename_meeting", { request: { id, title } });
}

export async function moveMeeting(
  id: string,
  folderId: string | null,
): Promise<MeetingRecord> {
  return invoke("move_meeting", { request: { id, folderId } });
}

export async function deleteMeeting(id: string): Promise<void> {
  return invoke("delete_meeting", { request: { id } });
}

export async function listMeetings(
  folderId?: string | null,
  limit = 50,
  offset = 0,
): Promise<MeetingListResponse> {
  return invoke("list_meetings", {
    request: { folderId: folderId ?? null, limit, offset },
  });
}

export async function getMeeting(id: string): Promise<MeetingRecord> {
  return invoke("get_meeting", { request: { id } });
}

export async function getActiveMeeting(): Promise<MeetingRecord | null> {
  return invoke("get_active_meeting");
}

export async function listMeetingSegments(
  meetingId: string,
  direction?: "outbound" | "inbound",
  fromSequence?: number,
  limit = 500,
): Promise<SegmentListResponse> {
  return invoke("list_meeting_segments", {
    request: {
      meetingId,
      direction: direction ?? null,
      fromSequence: fromSequence ?? null,
      beforeSequence: null,
      tail: null,
      limit,
    },
  });
}

export async function listMeetingSegmentsTail(
  meetingId: string,
  direction: "outbound" | "inbound",
  limit: number,
): Promise<SegmentListResponse> {
  return invoke("list_meeting_segments", {
    request: {
      meetingId,
      direction,
      fromSequence: null,
      beforeSequence: null,
      tail: true,
      limit,
    },
  });
}

export async function listMeetingSegmentsBefore(
  meetingId: string,
  direction: "outbound" | "inbound",
  beforeSequence: number,
  limit: number,
): Promise<SegmentListResponse> {
  return invoke("list_meeting_segments", {
    request: {
      meetingId,
      direction,
      fromSequence: null,
      beforeSequence: beforeSequence,
      tail: null,
      limit,
    },
  });
}

export async function searchSegments(options: {
  query: string;
  meetingId?: string | null;
  folderId?: string | null;
  limit?: number;
}): Promise<SegmentSearchHit[]> {
  return invoke("search_segments", {
    request: {
      query: options.query,
      meetingId: options.meetingId ?? null,
      folderId: options.folderId ?? null,
      limit: options.limit ?? null,
    },
  });
}

export async function searchMeetings(options: {
  query: string;
  folderId?: string | null;
  limit?: number;
}): Promise<MeetingSearchHit[]> {
  return invoke("search_meetings", {
    request: {
      query: options.query,
      folderId: options.folderId ?? null,
      limit: options.limit ?? null,
    },
  });
}

export type { SegmentCommittedEvent };

export async function getSegmentNeighbors(
  meetingId: string,
  segmentId: string,
): Promise<SegmentNeighbors> {
  return invoke("get_segment_neighbors", {
    request: { meetingId, segmentId },
  });
}

export async function getMeetingSummary(
  meetingId: string,
): Promise<MeetingSummary | null> {
  return invoke("get_meeting_summary", { request: { id: meetingId } });
}

export async function listSummaryTemplates(): Promise<SummaryTemplateInfo[]> {
  return invoke("list_summary_templates_cmd");
}

export async function listSummaryLanguages(): Promise<SummaryLanguageInfo[]> {
  return invoke("list_summary_languages_cmd");
}

export async function generateMeetingSummary(
  meetingId: string,
  templateId: string,
  language: string,
): Promise<MeetingSummary> {
  return invoke("generate_meeting_summary", {
    request: { id: meetingId, templateId, language },
  });
}

export async function getSummaryGenerationStatus(
  meetingId: string,
): Promise<SummaryGenerationStatus | null> {
  return invoke("get_summary_generation_status", {
    request: { id: meetingId },
  });
}

export async function updateMeetingSummary(
  meetingId: string,
  docJson: string,
): Promise<MeetingSummary> {
  return invoke("update_meeting_summary", {
    request: { meetingId, docJson },
  });
}

export async function listMeetingArtifacts(
  meetingId: string,
  kind?: ArtifactKind,
  status?: ArtifactStatus,
): Promise<MeetingArtifact[]> {
  return invoke("list_meeting_artifacts", {
    request: { meetingId, kind: kind ?? null, status: status ?? null },
  });
}

export async function setArtifactStatus(
  id: string,
  status: ArtifactStatus,
): Promise<MeetingArtifact> {
  return invoke("set_artifact_status", { request: { id, status } });
}

/** Edit text/owner/due. May re-key extracted rows into the user namespace. */
export async function updateMeetingArtifact(
  id: string,
  text: string,
  owner?: string | null,
  due?: string | null,
): Promise<MeetingArtifact> {
  return invoke("update_meeting_artifact", {
    request: { id, text, owner: owner ?? null, due: due ?? null },
  });
}

/** Manually add an artifact: origin `manual`, no citations. */
export async function createMeetingArtifact(
  meetingId: string,
  kind: ArtifactKind,
  text: string,
  owner?: string | null,
  due?: string | null,
): Promise<MeetingArtifact> {
  return invoke("create_meeting_artifact", {
    request: {
      meetingId,
      kind,
      text,
      owner: owner ?? null,
      due: due ?? null,
    },
  });
}

/** Hard-delete an artifact. */
export async function deleteMeetingArtifact(id: string): Promise<boolean> {
  return invoke("delete_meeting_artifact", { request: { id } });
}

export async function listMeetingEntities(
  meetingId: string,
): Promise<MeetingEntity[]> {
  return invoke("list_meeting_entities", { request: { id: meetingId } });
}

/** Rename / re-kind an entity. Keeps name-keyed id stable. */
export async function updateMeetingEntity(
  id: string,
  name: string,
  kind: string,
): Promise<MeetingEntity> {
  return invoke("update_meeting_entity", {
    request: { id, name, kind },
  });
}

/** Manually add a mentioned entity. Rejects duplicate names. */
export async function createMeetingEntity(
  meetingId: string,
  name: string,
  kind: string,
): Promise<MeetingEntity> {
  return invoke("create_meeting_entity", {
    request: { meetingId, name, kind },
  });
}

/** Hard-delete a mentioned entity. */
export async function deleteMeetingEntity(id: string): Promise<boolean> {
  return invoke("delete_meeting_entity", { request: { id } });
}

export type MeetingAudioChunk = {
  id: string;
  meetingId: string;
  direction: string;
  sequence: number;
  filePath: string;
  durationMs: number;
  sampleRate: number;
  startedAtMs: number;
  byteSize: number;
};

export type MeetingAudioSource = "room" | "you" | "meeting";

export type DecodeAudioWindow = {
  sampleRate: number;
  samples: number[];
};

export async function defaultMeetingAudioFolder(): Promise<string> {
  return invoke("default_meeting_audio_folder");
}

export async function pickMeetingAudioFolder(): Promise<string | null> {
  return invoke("pick_meeting_audio_folder");
}

export async function openMeetingAudioFolder(
  folder?: string | null,
): Promise<void> {
  return invoke("open_meeting_audio_folder", {
    folder: folder?.trim() ? folder : null,
  });
}

export type MissingAudioSpan = {
  startedAtMs: number;
  durationMs: number;
  direction: string;
};

export type VerifyMeetingAudioResult = {
  chunks: MeetingAudioChunk[];
  missingCount: number;
  recordedCount: number;
  missing: MissingAudioSpan[];
};

/** Check files on disk; zeros DB `byteSize` for missing chunks; returns playable only. */
export async function verifyMeetingAudio(
  meetingId: string,
): Promise<VerifyMeetingAudioResult> {
  return invoke("verify_meeting_audio", {
    request: { id: meetingId },
  });
}

export type MeetingAudioDiskUsage = {
  totalBytes: number;
};

export async function meetingAudioDiskUsage(): Promise<MeetingAudioDiskUsage> {
  return invoke("meeting_audio_disk_usage");
}

export async function decodeMeetingAudioWindow(
  meetingId: string,
  source: MeetingAudioSource,
  startMs: number,
  durationMs: number,
): Promise<DecodeAudioWindow> {
  return invoke("decode_meeting_audio_window", {
    request: {
      meetingId,
      source,
      startMs,
      durationMs,
    },
  });
}
