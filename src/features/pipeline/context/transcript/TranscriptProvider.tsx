import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  type ReactNode,
} from "react";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";

import type { TranscriptEvent } from "@/shared/lib/types/pipeline";
import { useLiveSegmentHydrate } from "../../hooks/useLiveSegmentHydrate";
import { getActiveMeeting } from "@/features/meeting/library/lib/meetingApi";
import type {
  SegmentCommittedEvent,
  SegmentPreviewEvent,
} from "@/features/meeting/library/lib/meetingTypes";
import {
  createTranscriptCoalesceScheduler,
  LIVE_TRANSCRIPT_FLUSH_MS,
} from "./liveTranscriptCoalesce";
import {
  liveTranscriptInitialState,
  liveTranscriptReducer,
  type DirectionTranscriptSlice,
  type LiveTranscriptAction,
} from "./liveTranscriptReducer";
import {
  CommittedContext,
  DispatchContext,
  InboundTranscriptContext,
  InterimContext,
  OutboundTranscriptContext,
} from "./transcriptContexts";

type CoalescedAction =
  | { type: "TRANSCRIPT"; event: TranscriptEvent }
  | { type: "SEGMENT_PREVIEW"; payload: SegmentPreviewEvent }
  | { type: "SEGMENT_COMMITTED"; payload: SegmentCommittedEvent };

function LiveSegmentHydrateInner() {
  useLiveSegmentHydrate();
  return null;
}

export function TranscriptProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(
    liveTranscriptReducer,
    liveTranscriptInitialState,
  );

  const coalesceRef = useRef(
    createTranscriptCoalesceScheduler<CoalescedAction>((batch) => {
      for (const action of batch) {
        dispatch(action);
      }
    }),
  );

  useEffect(() => {
    const coalesce = coalesceRef.current;
    return () => {
      coalesce.dispose();
    };
  }, []);

  const clearTranscripts = useCallback(() => {
    coalesceRef.current.clear();
    dispatch({ type: "CLEAR" });
  }, []);

  useEffect(() => {
    let cancelled = false;
    const coalesce = coalesceRef.current;

    const unlistenTranscript = listenSafe<TranscriptEvent>(
      APP_EVENTS.transcript,
      (event) => {
        if (cancelled) return;
        coalesce.enqueue({ type: "TRANSCRIPT", event: event.payload });
      },
    );
    const unlistenPreview = listenSafe<SegmentPreviewEvent>(
      APP_EVENTS.segmentPreview,
      (event) => {
        if (cancelled) return;
        coalesce.enqueue({ type: "SEGMENT_PREVIEW", payload: event.payload });
      },
    );
    const unlistenCommitted = listenSafe<SegmentCommittedEvent>(
      APP_EVENTS.segmentCommitted,
      (event) => {
        if (cancelled) return;
        coalesce.enqueue({ type: "SEGMENT_COMMITTED", payload: event.payload });
      },
    );

    return () => {
      cancelled = true;
      coalesce.clear();
      unlistenTranscript();
      unlistenPreview();
      unlistenCommitted();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    void getActiveMeeting().then((meeting) => {
      if (cancelled) return;
      dispatch({
        type: "SET_ACTIVE_MEETING",
        meetingId: meeting?.status === "live" ? meeting.id : null,
      });
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const dispatchValue = useMemo(
    () => ({
      dispatch: (action: LiveTranscriptAction) => {
        if (action.type === "CLEAR") {
          coalesceRef.current.clear();
        }
        dispatch(action);
      },
      activeMeetingId: state.activeMeetingId,
      clearTranscripts,
    }),
    [clearTranscripts, state.activeMeetingId],
  );

  const interimValue = state.interim;
  const committedValue = state.committed;

  const outboundSlice = useMemo(
    (): DirectionTranscriptSlice => ({
      interim: state.interim.outbound,
      committed: state.committed.outbound,
      liveSnapshot: state.liveSnapshot.outbound,
      hasMoreOlder: state.committed.outboundHasMoreOlder,
      loadingOlder: state.committed.loadingOlder.outbound,
    }),
    [
      state.interim.outbound,
      state.committed.outbound,
      state.liveSnapshot.outbound,
      state.committed.outboundHasMoreOlder,
      state.committed.loadingOlder.outbound,
    ],
  );

  const inboundSlice = useMemo(
    (): DirectionTranscriptSlice => ({
      interim: state.interim.inbound,
      committed: state.committed.inbound,
      liveSnapshot: state.liveSnapshot.inbound,
      hasMoreOlder: state.committed.inboundHasMoreOlder,
      loadingOlder: state.committed.loadingOlder.inbound,
    }),
    [
      state.interim.inbound,
      state.committed.inbound,
      state.liveSnapshot.inbound,
      state.committed.inboundHasMoreOlder,
      state.committed.loadingOlder.inbound,
    ],
  );

  return (
    <DispatchContext.Provider value={dispatchValue}>
      <InterimContext.Provider value={interimValue}>
        <CommittedContext.Provider value={committedValue}>
          <OutboundTranscriptContext.Provider value={outboundSlice}>
            <InboundTranscriptContext.Provider value={inboundSlice}>
              <LiveSegmentHydrateInner />
              {children}
            </InboundTranscriptContext.Provider>
          </OutboundTranscriptContext.Provider>
        </CommittedContext.Provider>
      </InterimContext.Provider>
    </DispatchContext.Provider>
  );
}

export { LIVE_TRANSCRIPT_FLUSH_MS };
