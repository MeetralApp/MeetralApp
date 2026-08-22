import { useCallback, useRef } from "react";

import { listMeetingSegmentsBefore } from "@/features/meeting/library/lib/meetingApi";
import { LIVE_SEGMENT_PAGE_SIZE } from "../lib/liveTranscriptConstants";
import type { TranscriptDirection } from "../lib/liveSegmentState";
import {
  useLiveCommitted,
  useLiveHasMoreOlder,
  useLiveLoadingOlder,
  useLiveTranscriptDispatch,
} from "@/features/pipeline/context/transcript/useLiveTranscript";

export function useLiveSegmentPagination(direction: TranscriptDirection) {
  const { dispatch, activeMeetingId } = useLiveTranscriptDispatch();
  const committed = useLiveCommitted(direction);
  const hasMoreOlder = useLiveHasMoreOlder(direction);
  const loadingOlder = useLiveLoadingOlder(direction);
  const inFlightRef = useRef(false);

  const loadOlder = useCallback(async () => {
    if (!activeMeetingId || !hasMoreOlder || loadingOlder || inFlightRef.current) {
      return;
    }
    if (committed.length === 0) return;

    const meetingIdAtStart = activeMeetingId;
    const minSequence = Math.min(...committed.map((s) => s.sequence));
    inFlightRef.current = true;
    dispatch({ type: "SET_LOADING_OLDER", direction, loading: true });

    try {
      const response = await listMeetingSegmentsBefore(
        meetingIdAtStart,
        direction,
        minSequence,
        LIVE_SEGMENT_PAGE_SIZE,
      );
      dispatch({
        type: "PREPEND_OLDER",
        meetingId: meetingIdAtStart,
        direction,
        segments: response.segments,
        hasMoreOlder: response.hasMoreOlder,
      });
    } finally {
      inFlightRef.current = false;
      dispatch({ type: "SET_LOADING_OLDER", direction, loading: false });
    }
  }, [
    activeMeetingId,
    committed,
    direction,
    dispatch,
    hasMoreOlder,
    loadingOlder,
  ]);

  return { loadOlder, loadingOlder, hasMoreOlder };
}
