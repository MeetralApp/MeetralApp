import { useCallback, useEffect, useState } from "react";

import { getActiveMeeting } from "../lib/meetingApi";
import type { MeetingRecord } from "../lib/meetingTypes";
import { useMeetingChanged } from "./useMeetingChanged";

export function useActiveMeeting() {
  const [activeMeeting, setActiveMeeting] = useState<MeetingRecord | null>(
    null,
  );
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const meeting = await getActiveMeeting();
      setActiveMeeting(meeting?.status === "live" ? meeting : null);
    } catch {
      setActiveMeeting(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useMeetingChanged(refresh);

  return { activeMeeting, loading, refresh };
}
