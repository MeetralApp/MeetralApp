import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { MicOff } from "lucide-react";

import EmptyState from "@/shared/components/EmptyState";
import { AppButton } from "@/shared/components/AppButton";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import {
  estimateTranscriptRowHeight,
  scrollWhenReady,
  type TranscriptScrollIntent,
} from "@/features/pipeline/lib/transcriptVirtualizer";
import { transcriptListPadClass } from "@/features/pipeline/lib/transcriptLayoutStyles";
import {
  TranscriptRow,
  type TranscriptRowVariant,
} from "./TranscriptTable";
import TimelineTranscriptSearch, {
  TimelineSearchTrigger,
} from "./TimelineTranscriptSearch";
import type {
  MeetingRecord,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";
import {
  filterSegmentsByDirection,
  formatSegmentTimestamp,
  sortSegmentsChronologically,
  timelineSpeakerLabel,
  type TimelineDirectionFilter,
} from "../lib/timelineTranscript";

const TIMELINE_META_HEIGHT = 22;

/** History detail always uses stacked Original / Translation for density. */
const TIMELINE_LAYOUT = "stacked" as const;

interface Props {
  meeting: MeetingRecord;
  segments: TranscriptSegment[];
  focusedSegmentId: string | null;
  scrollIntent: TranscriptScrollIntent | null;
  onScrollIntentConsumed?: () => void;
  transcriptVariant?: TranscriptRowVariant;
  onSegmentClick?: (segment: TranscriptSegment) => void;
  onReturnToLive?: () => void;
  onDeleteMeeting?: () => void;
  /** Controlled filter — synced with audio source in the player. */
  filter: TimelineDirectionFilter;
  onFilterChange: (next: TimelineDirectionFilter) => void;
  /** Extra bottom pad so floating chrome (audio player) does not cover rows. */
  bottomInsetClassName?: string;
}

function estimateTimelineRowHeight(
  segment: TranscriptSegment | undefined,
  variant: TranscriptRowVariant,
): number {
  return (
    estimateTranscriptRowHeight(segment, TIMELINE_LAYOUT, variant) +
    TIMELINE_META_HEIGHT
  );
}

export default function ReadonlyTranscriptTimeline({
  meeting,
  segments,
  focusedSegmentId,
  scrollIntent,
  onScrollIntentConsumed,
  transcriptVariant = "bilingual",
  onSegmentClick,
  onReturnToLive,
  onDeleteMeeting,
  filter,
  onFilterChange,
  bottomInsetClassName,
}: Props) {
  const notesMode = transcriptVariant === "notes";
  const scrollRef = useRef<HTMLDivElement>(null);
  const [searchOpen, setSearchOpen] = useState(false);

  useEffect(() => {
    setSearchOpen(false);
  }, [meeting.id]);

  const chronological = useMemo(
    () => sortSegmentsChronologically(segments),
    [segments],
  );
  const visibleSegments = useMemo(
    () => filterSegmentsByDirection(chronological, filter),
    [chronological, filter],
  );

  const virtualizer = useVirtualizer({
    count: visibleSegments.length,
    getScrollElement: () => scrollRef.current,
    getItemKey: (index) => visibleSegments[index]?.id ?? index,
    estimateSize: (index) =>
      estimateTimelineRowHeight(visibleSegments[index], transcriptVariant),
    overscan: 8,
  });

  const virtualItems = virtualizer.getVirtualItems();
  const anchorJumpPending =
    scrollIntent != null && scrollIntent.segmentId === focusedSegmentId;

  useEffect(() => {
    virtualizer.measure();
  }, [transcriptVariant, filter, virtualizer]);

  useLayoutEffect(() => {
    if (!scrollIntent) return;

    const index = visibleSegments.findIndex(
      (s) => s.id === scrollIntent.segmentId,
    );
    if (index < 0) {
      // Segment filtered out — switch to All so the jump can land.
      if (filter !== "all") {
        onFilterChange("all");
        return;
      }
      onScrollIntentConsumed?.();
      return;
    }

    const cancel = scrollWhenReady(scrollRef.current, () => {
      virtualizer.scrollToIndex(index, { align: "center", behavior: "auto" });
      onScrollIntentConsumed?.();
    });

    return cancel;
  }, [
    scrollIntent,
    visibleSegments,
    filter,
    onFilterChange,
    virtualizer,
    onScrollIntentConsumed,
  ]);

  useLayoutEffect(() => {
    if (anchorJumpPending || !focusedSegmentId) return;

    const index = visibleSegments.findIndex((s) => s.id === focusedSegmentId);
    if (index < 0) return;

    const cancel = scrollWhenReady(scrollRef.current, () => {
      virtualizer.scrollToIndex(index, { align: "center", behavior: "smooth" });
    });

    return cancel;
  }, [anchorJumpPending, focusedSegmentId, visibleSegments, virtualizer]);

  const emptyFilterCopy =
    filter === "outbound"
      ? notesMode
        ? "No notes from you in this meeting."
        : "No speech from you in this meeting."
      : filter === "inbound"
        ? notesMode
          ? "No notes from the meeting side."
          : "No speech from the meeting side."
        : "No segments to show.";

  const onJumpToSearchMatch = (segment: TranscriptSegment) => {
    if (filter !== "all" && segment.direction !== filter) {
      onFilterChange("all");
    }
    onSegmentClick?.(segment);
  };

  const floatingSearch = (
    <TimelineTranscriptSearch
      meetingId={meeting.id}
      segments={chronological}
      onJumpToSegment={onJumpToSearchMatch}
      open={searchOpen}
      onOpenChange={setSearchOpen}
    />
  );

  // Hide the Search chip while the find bar is open (bar has its own close).
  // right-4 ≈ top-2 + scrollbar gutter so the chip isn't flush to the rail.
  const searchTrigger =
    segments.length > 0 && !searchOpen ? (
      <div className="pointer-events-none absolute top-2 right-4 z-[4]">
        <div className="pointer-events-auto">
          <TimelineSearchTrigger
            open={false}
            onToggle={() => setSearchOpen(true)}
          />
        </div>
      </div>
    ) : null;

  if (segments.length === 0) {
    return (
      <div className="relative flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
        <div className="relative min-h-0 flex-1">
          <EmptyState
            icon={MicOff}
            title={notesMode ? "No notes recorded yet" : "No transcript recorded"}
            description={
              notesMode
                ? "This meeting ended without captured speech. Start Notes again to capture, or delete this empty entry."
                : "This meeting ended without any captured speech. Start a new meeting to record audio, or delete this empty entry."
            }
            action={
              <div className="flex flex-wrap justify-center gap-2">
                {onReturnToLive ? (
                  <Button type="button" size="sm" onClick={onReturnToLive}>
                    Start new meeting
                  </Button>
                ) : null}
                {onDeleteMeeting ? (
                  <AppButton
                    type="button"
                    destructive
                    size="sm"
                    onClick={onDeleteMeeting}
                  >
                    Delete meeting
                  </AppButton>
                ) : null}
              </div>
            }
          />
        </div>
      </div>
    );
  }

  return (
    <div className="relative flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
      <div className="relative min-h-0 flex-1 bg-card">
        {searchTrigger}
        {floatingSearch}
        <div className="h-full min-h-0 overflow-y-auto p-0" ref={scrollRef}>
          {visibleSegments.length === 0 ? (
            <div className="flex h-full min-h-[200px] items-center justify-center p-6 text-center">
              <p className="m-0 max-w-[28ch] text-sm text-muted-foreground">
                {emptyFilterCopy}
              </p>
            </div>
          ) : (
            <div
              className={cn(
                transcriptListPadClass,
                bottomInsetClassName,
                searchOpen && "pt-12",
              )}
            >
              <div
                className="relative w-full"
                style={{ height: `${virtualizer.getTotalSize()}px` }}
              >
                {virtualItems.length > 0 ? (
                  <div
                    className="absolute top-0 left-0 w-full"
                    style={{
                      transform: `translateY(${virtualItems[0]?.start ?? 0}px)`,
                    }}
                  >
                    {virtualItems.map((virtualRow) => {
                      const seg = visibleSegments[virtualRow.index];
                      if (!seg) return null;
                      const speaker = timelineSpeakerLabel(seg.direction);
                      const isYou = seg.direction === "outbound";
                      const timeLabel = formatSegmentTimestamp(seg.startedAtMs);
                      return (
                        <div
                          key={virtualRow.key}
                          data-index={virtualRow.index}
                          ref={virtualizer.measureElement}
                          className="pb-0.5"
                        >
                          <TranscriptRow
                            layout={TIMELINE_LAYOUT}
                            variant={transcriptVariant}
                            dataSegmentId={seg.id}
                            source={seg.sourceText}
                            translated={seg.translatedText}
                            connectionGap={seg.connectionGap}
                            focused={focusedSegmentId === seg.id}
                            interactive={!!onSegmentClick}
                            onClick={() => onSegmentClick?.(seg)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") onSegmentClick?.(seg);
                            }}
                            header={
                              <div
                                className="flex min-w-0 items-center gap-2"
                                aria-label={`${timeLabel}, ${speaker}`}
                              >
                                <span className="text-[0.7rem] tabular-nums leading-none text-muted-foreground">
                                  {timeLabel}
                                </span>
                                <span
                                  className={cn(
                                    "text-[0.65rem] font-medium leading-none tracking-wide",
                                    isYou
                                      ? "text-primary/80"
                                      : "text-muted-foreground",
                                  )}
                                >
                                  {speaker}
                                </span>
                              </div>
                            }
                          />
                        </div>
                      );
                    })}
                  </div>
                ) : null}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
