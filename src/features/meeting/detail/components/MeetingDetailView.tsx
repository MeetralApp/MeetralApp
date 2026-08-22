import { useCallback, useEffect, useState } from "react";

import MeetingDetailLayout from "./MeetingDetailLayout";
import MeetingDetailSkeleton from "./MeetingDetailSkeleton";
import { useMeetingDetail } from "../hooks/useMeetingDetail";
import { useMinDisplayDelay } from "@/shared/hooks/useMinDisplayDelay";
import { useMeetings } from "@/features/meeting/library/hooks/useMeetings";
import type {
  SegmentCitation,
  PendingScrollSegment,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";
import type { CitationAttrs } from "../lib/summaryDoc/citationExtension";
import type { AiProvider } from "@/shared/lib/types/pipeline";
import type { TranscriptScrollIntent } from "@/features/pipeline/lib/transcriptVirtualizer";

interface Props {
  meetingId: string;
  aiProvider: AiProvider;
  /** Intelligence answer language; "" = match the meeting's You language. */
  answerLanguage?: string;
  /** Meeting Intelligence Artifacts surface toggle. */
  artifactsEnabled?: boolean;
  pendingScrollSegment?: PendingScrollSegment | null;
  onPendingScrollConsumed?: () => void;
  onDeleted?: () => void;
  onReturnToLive?: () => void;
}

export default function MeetingDetailView({
  meetingId,
  aiProvider,
  answerLanguage = "",
  artifactsEnabled = true,
  pendingScrollSegment = null,
  onPendingScrollConsumed,
  onDeleted,
  onReturnToLive,
}: Props) {
  const {
    meeting,
    segments,
    summary,
    loading,
    summaryLoading,
    summaryProgress,
    error,
    summaryError,
    generateSummary,
    saveSummaryEdit,
    clearSummaryError,
  } = useMeetingDetail(meetingId);

  const { removeMeeting } = useMeetings();

  const [focusedSegmentId, setFocusedSegmentId] = useState<string | null>(null);
  const [scrollIntent, setScrollIntent] = useState<TranscriptScrollIntent | null>(
    null,
  );
  const [seekToMs, setSeekToMs] = useState<number | null>(null);

  const onSummaryCitationClick = useCallback((attrs: CitationAttrs) => {
    setFocusedSegmentId(attrs.segmentId);
    setScrollIntent({
      segmentId: attrs.segmentId,
      direction:
        attrs.direction === "outbound" ? "outbound" : "inbound",
    });
  }, []);

  const onScrollIntentConsumed = useCallback(() => {
    setScrollIntent(null);
  }, []);

  const onFocusSegment = useCallback((segment: TranscriptSegment) => {
    setFocusedSegmentId(segment.id);
    setScrollIntent({
      segmentId: segment.id,
      direction: segment.direction,
    });
  }, []);

  const onSeekFromSegment = useCallback((segment: TranscriptSegment) => {
    setFocusedSegmentId(segment.id);
    setScrollIntent({
      segmentId: segment.id,
      direction: segment.direction,
    });
    setSeekToMs(segment.startedAtMs);
  }, []);

  const onSeekConsumed = useCallback(() => {
    setSeekToMs(null);
  }, []);

  const onCitationClick = useCallback((citation: SegmentCitation) => {
    setFocusedSegmentId(citation.segmentId);
    setScrollIntent({
      segmentId: citation.segmentId,
      direction:
        citation.direction === "outbound" ? "outbound" : "inbound",
    });
    if (citation.startedAtMs != null) {
      setSeekToMs(citation.startedAtMs);
    }
  }, []);

  const handleDelete = useCallback(async () => {
    if (!meeting) return;
    await removeMeeting(meeting.id);
    onDeleted?.();
  }, [meeting, onDeleted, removeMeeting]);

  useEffect(() => {
    setFocusedSegmentId(null);
    setScrollIntent(null);
    setSeekToMs(null);
  }, [meetingId]);

  useEffect(() => {
    if (!pendingScrollSegment) return;
    if (pendingScrollSegment.meetingId !== meetingId) return;
    if (loading || segments.length === 0) return;
    const found = segments.find((s) => s.id === pendingScrollSegment.segmentId);
    if (!found) {
      onPendingScrollConsumed?.();
      return;
    }
    const direction =
      pendingScrollSegment.direction ??
      (found.direction === "outbound" ? "outbound" : "inbound");
    setFocusedSegmentId(found.id);
    setScrollIntent({ segmentId: found.id, direction });
    onPendingScrollConsumed?.();
  }, [
    pendingScrollSegment,
    meetingId,
    loading,
    segments,
    onPendingScrollConsumed,
  ]);

  const meetingLoading = loading && !meeting;
  const showMeetingSkeleton = useMinDisplayDelay(meetingLoading);

  if (showMeetingSkeleton) {
    return <MeetingDetailSkeleton />;
  }

  if (!meeting) {
    if (error) {
      return <p className="p-4 text-sm text-destructive">{error}</p>;
    }
    return <p className="p-4 text-sm text-muted-foreground">Meeting not found.</p>;
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <MeetingDetailLayout
        meeting={meeting}
        aiProvider={aiProvider}
        segments={segments}
        summary={summary}
        summaryLoading={summaryLoading}
        summaryProgress={summaryProgress}
        summaryError={summaryError}
        answerLanguage={answerLanguage}
        artifactsEnabled={artifactsEnabled}
        onGenerateSummary={(templateId, language) =>
          void generateSummary(templateId, language)
        }
        onSaveSummaryEdit={saveSummaryEdit}
        onDismissSummaryError={clearSummaryError}
        onReturnToLive={onReturnToLive}
        onDeleteMeeting={() => void handleDelete()}
        focusedSegmentId={focusedSegmentId}
        scrollIntent={scrollIntent}
        onScrollIntentConsumed={onScrollIntentConsumed}
        onFocusSegment={onFocusSegment}
        onSummaryCitationClick={onSummaryCitationClick}
        onCitationClick={onCitationClick}
        seekToMs={seekToMs}
        onSeekConsumed={onSeekConsumed}
        onSeekFromSegment={onSeekFromSegment}
      />
    </div>
  );
}
