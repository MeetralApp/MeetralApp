import type { TranscriptEvent } from "@/shared/lib/types/pipeline";
import { TRANSCRIPT_INTERIM_BUFFER } from "../../lib/liveTranscriptConstants";
import {
  createOptimisticSegment,
  mergeHydratedSegments,
  prependOlderSegments,
  replaceOptimisticOrAppend,
  sortSegmentsAsc,
  type TranscriptDirection,
} from "../../lib/liveSegmentState";
import type { SegmentCommittedEvent, SegmentPreviewEvent, TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  pruneInterimAfterSegmentCommit,
  segmentToUtteranceBlock,
  snapshotFromTranscriptEvent,
  type UtteranceBlock,
} from "../../lib/transcriptView";

function appendInterim(
  events: TranscriptEvent[],
  event: TranscriptEvent,
): TranscriptEvent[] {
  return [...events.slice(-TRANSCRIPT_INTERIM_BUFFER), event];
}

function emptyDirectionPair<T>(value: T): Record<TranscriptDirection, T> {
  return { outbound: value, inbound: value };
}

export interface CommittedSlice {
  outbound: TranscriptSegment[];
  inbound: TranscriptSegment[];
  outboundHasMoreOlder: boolean;
  inboundHasMoreOlder: boolean;
  outboundClientSeq: number;
  inboundClientSeq: number;
  loadingOlder: Record<TranscriptDirection, boolean>;
}

export interface InterimSlice {
  outbound: TranscriptEvent[];
  inbound: TranscriptEvent[];
}

interface LiveSnapshotSlice {
  outbound: UtteranceBlock | null;
  inbound: UtteranceBlock | null;
}

/** Per-direction slice — stable reference when that direction's data is unchanged. */
export interface DirectionTranscriptSlice {
  interim: TranscriptEvent[];
  committed: TranscriptSegment[];
  liveSnapshot: UtteranceBlock | null;
  hasMoreOlder: boolean;
  loadingOlder: boolean;
}

export interface LiveTranscriptStore {
  activeMeetingId: string | null;
  interim: InterimSlice;
  liveSnapshot: LiveSnapshotSlice;
  committed: CommittedSlice;
}

const initialCommitted: CommittedSlice = {
  outbound: [],
  inbound: [],
  outboundHasMoreOlder: false,
  inboundHasMoreOlder: false,
  outboundClientSeq: 0,
  inboundClientSeq: 0,
  loadingOlder: emptyDirectionPair(false),
};

const initialState: LiveTranscriptStore = {
  activeMeetingId: null,
  interim: { outbound: [], inbound: [] },
  liveSnapshot: { outbound: null, inbound: null },
  committed: initialCommitted,
};

export type LiveTranscriptAction =
  | { type: "SET_ACTIVE_MEETING"; meetingId: string | null }
  | { type: "TRANSCRIPT"; event: TranscriptEvent }
  | { type: "SEGMENT_PREVIEW"; payload: SegmentPreviewEvent }
  | { type: "SEGMENT_COMMITTED"; payload: SegmentCommittedEvent }
  | {
      type: "HYDRATE";
      meetingId: string;
      direction: TranscriptDirection;
      segments: TranscriptSegment[];
      hasMoreOlder: boolean;
    }
  | {
      type: "PREPEND_OLDER";
      meetingId: string;
      direction: TranscriptDirection;
      segments: TranscriptSegment[];
      hasMoreOlder: boolean;
    }
  | {
      type: "SET_LOADING_OLDER";
      direction: TranscriptDirection;
      loading: boolean;
    }
  | { type: "CLEAR" };

type Action = LiveTranscriptAction;

function resetTranscriptSlice(
  meetingId: string | null,
): Pick<LiveTranscriptStore, "activeMeetingId" | "interim" | "liveSnapshot" | "committed"> {
  return {
    activeMeetingId: meetingId,
    interim: { outbound: [], inbound: [] },
    liveSnapshot: emptyDirectionPair<UtteranceBlock | null>(null),
    committed: { ...initialCommitted },
  };
}

function mapSegment(segment: TranscriptSegment): TranscriptSegment {
  return {
    ...segment,
    direction: segment.direction as TranscriptDirection,
  };
}

function isEmptyBlock(block: UtteranceBlock): boolean {
  return (
    !block.source?.trim() &&
    !block.translated?.trim() &&
    !block.connectionGap
  );
}

function blocksMatchSegment(block: UtteranceBlock, segment: TranscriptSegment): boolean {
  const committed = segmentToUtteranceBlock(segment);
  const aSource = block.source?.trim() ?? "";
  const bSource = committed.source?.trim() ?? "";
  const aTranslated = block.translated?.trim() ?? "";
  const bTranslated = committed.translated?.trim() ?? "";
  return aSource === bSource && aTranslated === bTranslated;
}

function appendFromPreview(
  state: LiveTranscriptStore,
  preview: SegmentPreviewEvent,
): CommittedSlice {
  if (!state.activeMeetingId) {
    return state.committed;
  }

  const direction = preview.direction;
  const block: UtteranceBlock = {
    source: preview.sourceText,
    translated: preview.translatedText,
    connectionGap: preview.connectionGap,
  };
  if (isEmptyBlock(block)) {
    return state.committed;
  }

  const segments = state.committed[direction];
  const last = segments[segments.length - 1];
  if (last?.sequence === preview.sequence) {
    if (blocksMatchSegment(block, last)) {
      return state.committed;
    }
  }

  const clientSeqKey = `${direction}ClientSeq` as const;
  const clientSeq = state.committed[clientSeqKey];
  const optimistic = createOptimisticSegment(
    direction,
    clientSeq,
    state.activeMeetingId,
    preview.sequence,
    block.source ?? "",
    block.translated ?? "",
    block.connectionGap ?? false,
  );

  return {
    ...state.committed,
    [direction]: sortSegmentsAsc([...segments.filter((s) => s.sequence !== preview.sequence), optimistic]),
    [clientSeqKey]: clientSeq + 1,
  };
}

export function liveTranscriptReducer(
  state: LiveTranscriptStore,
  action: Action,
): LiveTranscriptStore {
  switch (action.type) {
    case "SET_ACTIVE_MEETING": {
      if (action.meetingId === state.activeMeetingId) {
        return state;
      }
      if (!action.meetingId) {
        return {
          ...state,
          ...resetTranscriptSlice(null),
        };
      }
      return {
        ...state,
        ...resetTranscriptSlice(action.meetingId),
      };
    }
    case "CLEAR":
      return { ...initialState };
    case "SET_LOADING_OLDER":
      return {
        ...state,
        committed: {
          ...state.committed,
          loadingOlder: {
            ...state.committed.loadingOlder,
            [action.direction]: action.loading,
          },
        },
      };
    case "HYDRATE": {
      if (!state.activeMeetingId || action.meetingId !== state.activeMeetingId) {
        return state;
      }
      if (action.direction === "outbound") {
        return {
          ...state,
          committed: {
            ...state.committed,
            outbound: mergeHydratedSegments(
              state.committed.outbound,
              action.segments,
            ),
            outboundHasMoreOlder: action.hasMoreOlder,
          },
        };
      }
      return {
        ...state,
        committed: {
          ...state.committed,
          inbound: mergeHydratedSegments(
            state.committed.inbound,
            action.segments,
          ),
          inboundHasMoreOlder: action.hasMoreOlder,
        },
      };
    }
    case "PREPEND_OLDER": {
      if (!state.activeMeetingId || action.meetingId !== state.activeMeetingId) {
        return state;
      }
      if (action.direction === "outbound") {
        return {
          ...state,
          committed: {
            ...state.committed,
            outbound: prependOlderSegments(
              state.committed.outbound,
              action.segments,
            ),
            outboundHasMoreOlder: action.hasMoreOlder,
            loadingOlder: {
              ...state.committed.loadingOlder,
              outbound: false,
            },
          },
        };
      }
      return {
        ...state,
        committed: {
          ...state.committed,
          inbound: prependOlderSegments(
            state.committed.inbound,
            action.segments,
          ),
          inboundHasMoreOlder: action.hasMoreOlder,
          loadingOlder: {
            ...state.committed.loadingOlder,
            inbound: false,
          },
        },
      };
    }
    case "SEGMENT_COMMITTED": {
      const { meetingId, segment } = action.payload;
      if (!state.activeMeetingId || meetingId !== state.activeMeetingId) {
        return state;
      }
      const dir = segment.direction as TranscriptDirection;
      const merged = replaceOptimisticOrAppend(
        state.committed[dir],
        mapSegment(segment),
      );
      return {
        ...state,
        committed: {
          ...state.committed,
          [dir]: merged,
        },
        interim: {
          ...state.interim,
          [dir]: pruneInterimAfterSegmentCommit(state.interim[dir]),
        },
        // Authoritative commit owns the row — drop stale live tail (Overlay parity).
        liveSnapshot: {
          ...state.liveSnapshot,
          [dir]: null,
        },
      };
    }
    case "SEGMENT_PREVIEW": {
      const preview = action.payload;
      if (!state.activeMeetingId || preview.meetingId !== state.activeMeetingId) {
        return state;
      }
      const dir = preview.direction as TranscriptDirection;
      // Mid-turn sentence commits keep the in-flight live tail; turn/gap clear it.
      const clearLive = preview.reason !== "sentence";
      return {
        ...state,
        committed: appendFromPreview(state, preview),
        liveSnapshot: clearLive
          ? { ...state.liveSnapshot, [dir]: null }
          : state.liveSnapshot,
      };
    }
    case "TRANSCRIPT": {
      const event = action.event;
      const dir = event.direction as TranscriptDirection;
      const interimDir = appendInterim(state.interim[dir], event);

      // Turn/gap end: live→committed is owned by segment-preview (Overlay parity).
      if (event.turnComplete || event.connectionGap) {
        return {
          ...state,
          interim: { ...state.interim, [dir]: interimDir },
          liveSnapshot: {
            ...state.liveSnapshot,
            [dir]: null,
          },
        };
      }

      const snapshot = snapshotFromTranscriptEvent(event);

      return {
        ...state,
        interim: { ...state.interim, [dir]: interimDir },
        liveSnapshot: {
          ...state.liveSnapshot,
          [dir]: snapshot,
        },
      };
    }
    default:
      return state;
  }
}

export const liveTranscriptInitialState = initialState;
