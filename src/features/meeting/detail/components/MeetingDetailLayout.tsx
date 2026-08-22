import { useCallback, useRef, useState } from "react";
import { Panel, PanelGroup } from "react-resizable-panels";

import ArtifactsPanel from "./ArtifactsPanel";
import MeetingAudioPlayer from "./MeetingAudioPlayer";
import ReadonlyTranscriptTimeline from "./ReadonlyTranscriptTimeline";
import SummaryPanel from "./SummaryPanel";
import { cn } from "@/shared/lib/utils";
import { isNotesSession } from "@/features/pipeline/lib/sessionMode";
import { pipelineToolbarClasses } from "@/features/pipeline/lib/pipelineColors";
import TranscriptResizeHandle from "@/features/pipeline/components/TranscriptResizeHandle";
import type { CitationAttrs } from "../lib/summaryDoc/citationExtension";
import type {
  SegmentCitation,
  MeetingRecord,
  MeetingSummary,
  SummaryProgressUi,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";
import type { AiProvider } from "@/shared/lib/types/pipeline";
import type { TranscriptScrollIntent } from "@/features/pipeline/lib/transcriptVirtualizer";
import {
  audioSourceForFilter,
  type TimelineDirectionFilter,
} from "../lib/timelineTranscript";

interface Props {
  meeting: MeetingRecord;
  aiProvider: AiProvider;
  segments: TranscriptSegment[];
  summary: MeetingSummary | null;
  summaryLoading: boolean;
  summaryProgress?: SummaryProgressUi | null;
  summaryError: string | null;
  /** Intelligence answer language; "" = match the meeting's You language. */
  answerLanguage?: string;
  /** Meeting Intelligence Artifacts surface toggle. */
  artifactsEnabled?: boolean;
  onGenerateSummary: (templateId: string, language: string) => void;
  onSaveSummaryEdit: (docJson: string) => Promise<void>;
  onDismissSummaryError: () => void;
  onReturnToLive?: () => void;
  onDeleteMeeting?: () => void;
  focusedSegmentId: string | null;
  scrollIntent: TranscriptScrollIntent | null;
  onScrollIntentConsumed: () => void;
  onFocusSegment: (segment: TranscriptSegment) => void;
  onSummaryCitationClick: (attrs: CitationAttrs) => void;
  onCitationClick: (citation: SegmentCitation) => void;
  seekToMs: number | null;
  onSeekConsumed: () => void;
  onSeekFromSegment: (segment: TranscriptSegment) => void;
}

export default function MeetingDetailLayout({
  meeting,
  aiProvider,
  segments,
  summary,
  summaryLoading,
  summaryProgress,
  summaryError,
  answerLanguage = "",
  artifactsEnabled = true,
  onGenerateSummary,
  onSaveSummaryEdit,
  onDismissSummaryError,
  onReturnToLive,
  onDeleteMeeting,
  focusedSegmentId,
  scrollIntent,
  onScrollIntentConsumed,
  onFocusSegment,
  onSummaryCitationClick,
  onCitationClick,
  seekToMs,
  onSeekConsumed,
  onSeekFromSegment,
}: Props) {
  const notesMode = isNotesSession(meeting);
  const [timelineFilter, setTimelineFilter] =
    useState<TimelineDirectionFilter>("all");
  const audioSource = audioSourceForFilter(timelineFilter);
  const citeInserterRef = useRef<((segment: TranscriptSegment) => void) | null>(
    null,
  );
  const [docEditing, setDocEditing] = useState(false);

  const registerCiteInserter = useCallback(
    (inserter: ((segment: TranscriptSegment) => void) | null) => {
      citeInserterRef.current = inserter;
    },
    [],
  );

  const onSegmentClick = useCallback(
    (segment: TranscriptSegment) => {
      if (docEditing) {
        citeInserterRef.current?.(segment);
      }
      onSeekFromSegment(segment);
    },
    [docEditing, onSeekFromSegment],
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-0 p-0">
      <PanelGroup
        direction="horizontal"
        autoSaveId="meeting-detail-split-v1"
        className={cn(
          "min-h-0 flex-1 overflow-hidden rounded-none border-0 bg-transparent",
          "[&_[data-panel]]:flex [&_[data-panel]]:min-h-0 [&_[data-panel]]:min-w-0 [&_[data-panel]]:overflow-hidden",
          "[&_[data-panel]>_*]:min-h-0 [&_[data-panel]>_*]:min-w-0 [&_[data-panel]>_*]:flex-1",
        )}
      >
        <Panel id="summary" minSize={28} defaultSize={38} order={1}>
          <section
            className={cn(
              "relative flex h-full min-h-0 min-w-0 flex-col overflow-hidden",
              pipelineToolbarClasses.paneSurface,
            )}
            aria-label={notesMode ? "Meeting notes" : "Summary"}
          >
            <SummaryPanel
              meeting={meeting}
              aiProvider={aiProvider}
              segments={segments}
              summary={summary}
              summaryLoading={summaryLoading}
              summaryProgress={summaryProgress}
              summaryError={summaryError}
              answerLanguage={answerLanguage}
              onGenerate={onGenerateSummary}
              onSaveEdit={onSaveSummaryEdit}
              onDismissError={onDismissSummaryError}
              onCitationClick={onSummaryCitationClick}
              onReturnToLive={onReturnToLive}
              onDocEditingChange={setDocEditing}
              registerCiteInserter={registerCiteInserter}
              footer={
                artifactsEnabled ? (
                  <ArtifactsPanel
                    meeting={meeting}
                    summary={summary}
                    onCitationClick={onCitationClick}
                    segments={segments}
                  />
                ) : null
              }
            />
          </section>
        </Panel>

        <TranscriptResizeHandle />

        <Panel id="transcript" minSize={28} defaultSize={62} order={3}>
          <section
            className={cn(
              "relative flex h-full min-h-0 min-w-0 flex-col overflow-hidden",
              pipelineToolbarClasses.paneSurface,
            )}
            aria-label={notesMode ? "Notes" : "Transcript"}
          >
            <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-card">
              <ReadonlyTranscriptTimeline
                meeting={meeting}
                segments={segments}
                focusedSegmentId={focusedSegmentId}
                scrollIntent={scrollIntent}
                onScrollIntentConsumed={onScrollIntentConsumed}
                onSegmentClick={onSegmentClick}
                onReturnToLive={onReturnToLive}
                onDeleteMeeting={onDeleteMeeting}
                transcriptVariant={notesMode ? "notes" : "bilingual"}
                filter={timelineFilter}
                onFilterChange={setTimelineFilter}
                bottomInsetClassName="pb-20"
              />
            </div>
            <div className="pointer-events-none absolute inset-x-0 bottom-0 z-10 p-2.5 pt-0">
              <div className="pointer-events-auto">
                <MeetingAudioPlayer
                  meetingId={meeting.id}
                  segments={segments}
                  audioSource={audioSource}
                  filter={timelineFilter}
                  onFilterChange={setTimelineFilter}
                  onFocusSegment={onFocusSegment}
                  seekToMs={seekToMs}
                  onSeekConsumed={onSeekConsumed}
                />
              </div>
            </div>
          </section>
        </Panel>
      </PanelGroup>
    </div>
  );
}
