import { useContext, useMemo } from "react";

import type { TranscriptEvent } from "@/shared/lib/types/pipeline";
import type { TranscriptDirection } from "../../lib/liveSegmentState";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import type { UtteranceBlock } from "../../lib/transcriptView";
import type { DirectionTranscriptSlice } from "./liveTranscriptReducer";
import {
  DispatchContext,
  InboundTranscriptContext,
  InterimContext,
  OutboundTranscriptContext,
} from "./transcriptContexts";

function useDispatchContext() {
  const ctx = useContext(DispatchContext);
  if (!ctx) {
    throw new Error("useDispatchContext must be used within TranscriptProvider");
  }
  return ctx;
}

function useDirectionSlice(
  direction: TranscriptDirection,
): DirectionTranscriptSlice {
  const ctx = useContext(
    direction === "outbound"
      ? OutboundTranscriptContext
      : InboundTranscriptContext,
  );
  if (!ctx) {
    throw new Error("useDirectionSlice must be used within TranscriptProvider");
  }
  return ctx;
}

export function useLiveInterim(direction: TranscriptDirection): TranscriptEvent[] {
  return useDirectionSlice(direction).interim;
}

export function useLiveSnapshot(direction: TranscriptDirection): UtteranceBlock | null {
  return useDirectionSlice(direction).liveSnapshot;
}

export function useLiveCommitted(direction: TranscriptDirection): TranscriptSegment[] {
  return useDirectionSlice(direction).committed;
}

export function useLiveHasMoreOlder(direction: TranscriptDirection): boolean {
  return useDirectionSlice(direction).hasMoreOlder;
}

export function useLiveLoadingOlder(direction: TranscriptDirection): boolean {
  return useDirectionSlice(direction).loadingOlder;
}

export function useLiveTranscriptDispatch() {
  return useDispatchContext();
}

/** @deprecated Use useLiveInterim / useLiveCommitted — kept for App clearTranscripts */
export function useTranscripts() {
  const { clearTranscripts } = useDispatchContext();
  const interim = useContext(InterimContext);
  if (!interim) {
    throw new Error("useTranscripts must be used within TranscriptProvider");
  }
  const transcripts = useMemo(
    () => [...interim.outbound, ...interim.inbound],
    [interim.inbound, interim.outbound],
  );
  return { transcripts, clearTranscripts };
}
