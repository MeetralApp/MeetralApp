import { useCallback, useEffect, useRef } from "react";

import { getActiveMeeting, listMeetingSegmentsTail } from "@/features/meeting/library/lib/meetingApi";
import { LIVE_SEGMENT_INITIAL_WINDOW } from "../lib/liveTranscriptConstants";
import type { TranscriptDirection } from "../lib/liveSegmentState";
import { useLiveTranscriptDispatch } from "@/features/pipeline/context/transcript/useLiveTranscript";
import { useMeetingChanged } from "@/features/meeting/library/hooks/useMeetingChanged";

const DIRECTIONS: TranscriptDirection[] = ["outbound", "inbound"];

export function useLiveSegmentHydrate() {
  const { dispatch, activeMeetingId } = useLiveTranscriptDispatch();
  const hydratedForMeeting = useRef<string | null>(null);
  const generationRef = useRef(0);

  const hydrate = useCallback(
    async (meetingId: string) => {
      if (hydratedForMeeting.current === meetingId) return;
      hydratedForMeeting.current = meetingId;
      generationRef.current += 1;
      const gen = generationRef.current;

      await Promise.all(
        DIRECTIONS.map(async (direction) => {
          const response = await listMeetingSegmentsTail(
            meetingId,
            direction,
            LIVE_SEGMENT_INITIAL_WINDOW,
          );
          if (gen !== generationRef.current) return;
          dispatch({
            type: "HYDRATE",
            meetingId,
            direction,
            segments: response.segments,
            hasMoreOlder: response.hasMoreOlder,
          });
        }),
      );
    },
    [dispatch],
  );

  useEffect(() => {
    if (!activeMeetingId) {
      hydratedForMeeting.current = null;
      generationRef.current += 1;
      return;
    }
    void hydrate(activeMeetingId);
  }, [activeMeetingId, hydrate]);

  useMeetingChanged(() => {
    void getActiveMeeting().then((meeting) => {
      const id = meeting?.status === "live" ? meeting.id : null;
      dispatch({ type: "SET_ACTIVE_MEETING", meetingId: id });
      if (id) {
        hydratedForMeeting.current = null;
        generationRef.current += 1;
        void hydrate(id);
      }
    });
  });
}
