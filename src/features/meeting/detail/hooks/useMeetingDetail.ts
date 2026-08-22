import { useCallback, useEffect, useRef, useState } from "react";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";

import {
  generateMeetingSummary,
  getMeeting,
  getMeetingSummary,
  getSummaryGenerationStatus,
  listMeetingSegments,
  updateMeetingSummary,
} from "@/features/meeting/library/lib/meetingApi";
import {
  formatSummaryError,
  formatSummaryProgress,
  formatSummaryProgressLabel,
} from "@/features/meeting/library/lib/meetingDisplay";
import { dedupeSegmentsById } from "@/features/pipeline/lib/liveSegmentState";
import type {
  MeetingRecord,
  MeetingSummary,
  SegmentCommittedEvent,
  SummaryDoneEvent,
  SummaryErrorEvent,
  SummaryProgressEvent,
  SummaryProgressUi,
  SummaryGenerationStatus,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";

function progressFromStatus(status: SummaryGenerationStatus): SummaryProgressUi {
  return {
    label: formatSummaryProgressLabel(status.phase),
    detail: formatSummaryProgress(status.phase, status.current, status.total),
  };
}

export function useMeetingDetail(meetingId: string | null) {
  const [meeting, setMeeting] = useState<MeetingRecord | null>(null);
  const [segments, setSegments] = useState<TranscriptSegment[]>([]);
  const [summary, setSummary] = useState<MeetingSummary | null>(null);
  const summaryRef = useRef<MeetingSummary | null>(null);
  summaryRef.current = summary;
  const [loading, setLoading] = useState(false);
  const [summaryLoading, setSummaryLoading] = useState(false);
  const [summaryProgress, setSummaryProgress] =
    useState<SummaryProgressUi | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [summaryError, setSummaryError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!meetingId) {
      setMeeting(null);
      setSegments([]);
      setSummary(null);
      setSummaryError(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const [m, segRes] = await Promise.all([
        getMeeting(meetingId),
        listMeetingSegments(meetingId, undefined, undefined, 2000),
      ]);
      setMeeting(m);
      setSegments(segRes.segments);
      const sum = await getMeetingSummary(meetingId);
      setSummary(sum);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!meetingId) return;

    const unlistenCommitted = listenSafe<SegmentCommittedEvent>(
      APP_EVENTS.segmentCommitted,
      (event) => {
        if (event.payload.meetingId !== meetingId) return;
        setSegments((prev) =>
          dedupeSegmentsById([...prev, event.payload.segment]),
        );
      },
    );

    const unlistenUpdated = listenSafe<MeetingRecord>(
      APP_EVENTS.meetingUpdated,
      (event) => {
        if (event.payload.id !== meetingId) return;
        setMeeting(event.payload);
      },
    );

    return () => {
      unlistenCommitted();
      unlistenUpdated();
    };
  }, [meetingId]);

  // Restore + follow a background summary generation across remounts.
  useEffect(() => {
    if (!meetingId) return;

    let cancelled = false;

    getSummaryGenerationStatus(meetingId)
      .then((status) => {
        if (cancelled || !status) return;
        // Don't restore if summary already loaded (race with summary-done).
        if (summaryRef.current) return;
        setSummaryLoading(true);
        setSummaryError(null);
        setSummaryProgress(progressFromStatus(status));
      })
      .catch(() => {
      // Ignore restore errors; the listener path still works.
      });

    const unlistenProgress = listenSafe<SummaryProgressEvent>(
      APP_EVENTS.summaryProgress,
      (event) => {
        if (event.payload.meetingId !== meetingId) return;
        const { phase, current, total, partialText } = event.payload;
        setSummaryProgress({
          label: formatSummaryProgressLabel(phase),
          detail: formatSummaryProgress(phase, current, total),
          partialText,
        });
      },
    );

    const unlistenDone = listenSafe<SummaryDoneEvent>(
      APP_EVENTS.summaryDone,
      (event) => {
        if (event.payload.meetingId !== meetingId) return;
        // Skip refetch if this view already set the summary from the
        // generate command return.
        if (summaryRef.current) {
          setSummaryLoading(false);
          setSummaryProgress(null);
          return;
        }
        void getMeetingSummary(meetingId)
          .then((sum) => setSummary(sum))
          .catch(() => undefined)
          .finally(() => {
            setSummaryLoading(false);
            setSummaryProgress(null);
          });
      },
    );

    const unlistenError = listenSafe<SummaryErrorEvent>(
      APP_EVENTS.summaryError,
      (event) => {
        if (event.payload.meetingId !== meetingId) return;
        setSummaryError(formatSummaryError(event.payload.message));
        setSummaryLoading(false);
        setSummaryProgress(null);
      },
    );

    return () => {
      cancelled = true;
      unlistenProgress();
      unlistenDone();
      unlistenError();
    };
  }, [meetingId]);

  const refreshMetadata = useCallback(async () => {
    if (!meetingId) return;
    try {
      const m = await getMeeting(meetingId);
      setMeeting(m);
    } catch (e) {
      setError(String(e));
    }
  }, [meetingId]);

  const generateSummary = useCallback(
    async (templateId: string, language: string) => {
      if (!meetingId) return;
      setSummaryLoading(true);
      setSummaryError(null);
      setSummaryProgress(null);

      try {
        const sum = await generateMeetingSummary(
          meetingId,
          templateId,
          language,
        );
        setSummary(sum);
        setSummaryError(null);
      } catch (e) {
        setSummaryError(formatSummaryError(String(e)));
      } finally {
        setSummaryLoading(false);
        setSummaryProgress(null);
      }
    },
    [meetingId],
  );

  const saveSummaryEdit = useCallback(
    async (docJson: string) => {
      if (!meetingId) return;
      setSummaryError(null);
      try {
        const sum = await updateMeetingSummary(meetingId, docJson);
        setSummary(sum);
      } catch (e) {
        setSummaryError(formatSummaryError(String(e)));
        throw e;
      }
    },
    [meetingId],
  );

  const clearSummaryError = useCallback(() => {
    setSummaryError(null);
  }, []);

  return {
    meeting,
    segments,
    summary,
    loading,
    summaryLoading,
    summaryProgress,
    error,
    summaryError,
    refresh,
    refreshMetadata,
    generateSummary,
    saveSummaryEdit,
    clearSummaryError,
  };
}
