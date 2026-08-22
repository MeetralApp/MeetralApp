import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowDown, Loader2, Mic, Settings, Volume2 } from "lucide-react";

import {
  useLiveCommitted,
  useLiveHasMoreOlder,
  useLiveInterim,
  useLiveLoadingOlder,
  useLiveSnapshot,
} from "@/features/pipeline/context/transcript/useLiveTranscript";
import { useStickToBottomScroll } from "@/shared/hooks/useStickToBottomScroll";
import { useLiveSegmentPagination } from "../hooks/useLiveSegmentPagination";
import {
  LOAD_MORE_INDEX_THRESHOLD,
  LIVE_LOAD_MORE_TOP_THRESHOLD_PX,
} from "../lib/liveTranscriptConstants";
import type { TranscriptDirection } from "../lib/liveSegmentState";
import type { TranscriptLayout } from "@/shared/lib/types/pipeline";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  buildLiveDisplayState,
  liveDisplayRowKey,
  liveTailFingerprint,
  segmentToUtteranceBlock,
  type LiveDisplayRow,
} from "../lib/transcriptView";
import {
  estimateLiveDisplayRowHeight,
  scrollWhenReady,
} from "../lib/transcriptVirtualizer";
import { listenTranscriptFocus } from "../lib/transcriptFocus";
import { transcriptListPadClass } from "../lib/transcriptLayoutStyles";
import {
  TranscriptRow,
  TranscriptTableHeader,
  type TranscriptRowVariant,
} from "@/features/meeting/detail/components/TranscriptTable";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import EmptyState from "@/shared/components/EmptyState";
import type { ColumnIdleBadgeSnapshot } from "../lib/appState";
import { pipelineToolbarClasses } from "../lib/pipelineColors";

/** Citation/anchor flash — clears shell emphasis without re-pinning live. */
const CITATION_FOCUS_MS = 2_000;
const CommittedTranscriptRowCell = memo(function CommittedTranscriptRowCell({
  segment,
  layout,
  variant,
  focused,
}: {
  segment: TranscriptSegment;
  layout: TranscriptLayout;
  variant: TranscriptRowVariant;
  focused?: boolean;
}) {
  const block = segmentToUtteranceBlock(segment);
  return (
    <div className="group/row relative">
      <TranscriptRow
        layout={layout}
        variant={variant}
        dataSegmentId={segment.id}
        source={block.source}
        translated={block.translated}
        connectionGap={block.connectionGap}
        focused={focused}
      />
    </div>
  );
});

const LiveTailRowCell = memo(function LiveTailRowCell({
  source,
  translated,
  connectionGap,
  layout,
  variant,
}: {
  source?: string;
  translated?: string;
  connectionGap?: boolean;
  layout: TranscriptLayout;
  variant: TranscriptRowVariant;
}) {
  return (
    <TranscriptRow
      layout={layout}
      variant={variant}
      live
      source={source}
      translated={translated}
      connectionGap={connectionGap}
    />
  );
});

const LiveTranscriptRowCell = memo(function LiveTranscriptRowCell({
  row,
  layout,
  variant,
  focused,
}: {
  row: LiveDisplayRow;
  layout: TranscriptLayout;
  variant: TranscriptRowVariant;
  focused?: boolean;
}) {
  if (row.kind === "live") {
    return (
      <LiveTailRowCell
        layout={layout}
        variant={variant}
        source={row.block.source}
        translated={row.block.translated}
        connectionGap={row.block.connectionGap}
      />
    );
  }

  return (
    <CommittedTranscriptRowCell
      segment={row.segment}
      layout={layout}
      variant={variant}
      focused={focused}
    />
  );
});

const LiveTranscriptVirtualList = memo(function LiveTranscriptVirtualList({
  direction,
  displayRows,
  rowCount,
  liveTailText,
  layout,
  variant,
  scrollRef,
  isPinned,
  followContent,
  beginProgrammaticScroll,
  focusedSegmentId,
  scrollIntentSegmentId,
  onScrollIntentConsumed,
  onRangeNearTop,
}: {
  direction: TranscriptDirection;
  displayRows: LiveDisplayRow[];
  rowCount: number;
  liveTailText: string | null;
  layout: TranscriptLayout;
  variant: TranscriptRowVariant;
  scrollRef: React.RefObject<HTMLDivElement | null>;
  isPinned: boolean;
  /** When false (pipeline idle), do not follow live tail / row growth. */
  followContent: boolean;
  /** Arm stick echo-guard around scrollToIndex. */
  beginProgrammaticScroll: () => void;
  focusedSegmentId: string | null;
  /** One-shot jump target; cleared after scrollToIndex. */
  scrollIntentSegmentId: string | null;
  onScrollIntentConsumed: () => void;
  onRangeNearTop: () => void;
}) {
  const loadingOlder = useLiveLoadingOlder(direction);
  // Start at 0 / unpinned so the first pinned layout pass aligns to the live end
  // (hook applyScroll is false — VirtualList owns all stick scrolling).
  const prevRowCountRef = useRef(0);
  const prevPinnedRef = useRef(false);

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    getItemKey: (index) => liveDisplayRowKey(displayRows[index]!, direction),
    estimateSize: (index) =>
      estimateLiveDisplayRowHeight(displayRows[index], layout, variant),
    overscan: 8,
  });

  const virtualItems = virtualizer.getVirtualItems();

  useEffect(() => {
    virtualizer.measure();
  }, [layout, variant, virtualizer]);

  const scrollToLiveEnd = useCallback(() => {
    if (rowCount === 0) return;
    beginProgrammaticScroll();
    // Prefer scrollToIndex over scrollTop=scrollHeight — virtual totalSize from
    // estimates can overshoot and leave a large empty gap below the last segment.
    virtualizer.scrollToIndex(rowCount - 1, { align: "end" });
  }, [beginProgrammaticScroll, rowCount, virtualizer]);

  useLayoutEffect(() => {
    const countGrew = rowCount > prevRowCountRef.current;
    const pinnedNow = isPinned && !prevPinnedRef.current;
    prevRowCountRef.current = rowCount;
    prevPinnedRef.current = isPinned;

    // Skip while a citation jump is in flight — avoids racing re-pin → live.
    if (!isPinned || rowCount === 0 || scrollIntentSegmentId) return;

    // Jump to live / re-pin: always align end (even when followContent is false).
    if (pinnedNow) {
      scrollToLiveEnd();
      return;
    }
    // Live follow: only on row growth while content-follow is armed.
    if (!followContent || !countGrew) return;
    scrollToLiveEnd();
  }, [
    followContent,
    isPinned,
    rowCount,
    scrollIntentSegmentId,
    scrollToLiveEnd,
  ]);

  useLayoutEffect(() => {
    if (!isPinned || !followContent || liveTailText == null || scrollIntentSegmentId) {
      return;
    }
    scrollToLiveEnd();
  }, [
    followContent,
    isPinned,
    liveTailText,
    scrollIntentSegmentId,
    scrollToLiveEnd,
  ]);

  useLayoutEffect(() => {
    if (!scrollIntentSegmentId) return;

    const index = displayRows.findIndex(
      (row) =>
        row.kind === "committed" && row.segment.id === scrollIntentSegmentId,
    );
    if (index < 0) {
      onScrollIntentConsumed();
      return;
    }

    const cancel = scrollWhenReady(scrollRef.current, () => {
      beginProgrammaticScroll();
      // `auto` (not smooth): fewer near-bottom scroll frames that could re-pin
      // before unpin-hold arms / settles.
      virtualizer.scrollToIndex(index, { align: "center", behavior: "auto" });
      onScrollIntentConsumed();
    });
    return cancel;
  }, [
    beginProgrammaticScroll,
    scrollIntentSegmentId,
    displayRows,
    virtualizer,
    scrollRef,
    onScrollIntentConsumed,
  ]);

  useEffect(() => {
    const first = virtualItems[0];
    if (first && first.index <= LOAD_MORE_INDEX_THRESHOLD) {
      onRangeNearTop();
    }
  }, [virtualItems, onRangeNearTop]);

  if (rowCount === 0) {
    return null;
  }

  const rowsToRender =
    virtualItems.length > 0
      ? virtualItems
      : displayRows.map((_, index) => ({
          index,
          key: liveDisplayRowKey(displayRows[index]!, direction),
          start: 0,
          size: estimateLiveDisplayRowHeight(displayRows[index], layout, variant),
        }));

  const renderRow = (virtualRow: {
    index: number;
    key: string | number | bigint;
  }) => {
    const row = displayRows[virtualRow.index];
    if (!row) return null;
    const focused =
      row.kind === "committed" &&
      focusedSegmentId != null &&
      row.segment.id === focusedSegmentId;
    return (
      <div
        key={virtualRow.key}
        data-index={virtualRow.index}
        data-segment-id={
          row.kind === "committed" ? row.segment.id : undefined
        }
        ref={virtualizer.measureElement}
      >
        <LiveTranscriptRowCell
          row={row}
          layout={layout}
          variant={variant}
          focused={focused}
        />
      </div>
    );
  };

  return (
    <>
      {loadingOlder ? (
        <div className="flex items-center gap-2 px-1 py-2 text-xs text-muted-foreground">
          <Loader2 className="size-3.5 animate-spin" aria-hidden />
          Loading earlier transcript…
        </div>
      ) : null}
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
            {rowsToRender.map(renderRow)}
          </div>
        ) : (
          <div className="w-full">{rowsToRender.map(renderRow)}</div>
        )}
      </div>
    </>
  );
});

export interface LiveTranscriptColumnShellProps {
  title: string;
  direction: TranscriptDirection;
  placeholder: string;
  transcriptLayout: TranscriptLayout;
  /** Notes: single-text rows (no Original | Translation). */
  transcriptVariant?: TranscriptRowVariant;
  /**
  * Elevated card chrome (radius + shadow). Notes unified shell sets false so
  * columns sit flush inside the shared outer surface.
  */
  elevated?: boolean;
  toolbar: ReactNode;
  banners: ReactNode;
  idleBadge?: ColumnIdleBadgeSnapshot;
  onOpenSettings?: () => void;
  /**
  * When true (default), stick-to-bottom follows new content while pinned.
  * Pass false when this direction's pipeline is idle so stop-translate does
  * not keep yanking the viewport to the live tail.
  */
  followContent?: boolean;
}

function LiveTranscriptColumnInner({
  direction,
  placeholder,
  transcriptLayout,
  transcriptVariant = "bilingual",
  elevated = true,
  toolbar,
  banners,
  idleBadge,
  onOpenSettings,
  followContent = true,
}: LiveTranscriptColumnShellProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const committed = useLiveCommitted(direction);
  const interim = useLiveInterim(direction);
  const liveSnapshot = useLiveSnapshot(direction);
  const hasMoreOlder = useLiveHasMoreOlder(direction);
  const { loadOlder, loadingOlder } = useLiveSegmentPagination(direction);

  const { rows: displayRows, live } = useMemo(
    () => buildLiveDisplayState(committed, liveSnapshot, interim),
    [committed, liveSnapshot, interim],
  );
  const rowCount = displayRows.length;
  const liveTailText = useMemo(
    () => liveTailFingerprint(displayRows),
    [displayRows],
  );
  const isEmpty = rowCount === 0;
  const columnId =
    direction === "outbound" ? "transcript-outbound" : "transcript-inbound";

  const [focusedSegmentId, setFocusedSegmentId] = useState<string | null>(
    null,
  );
  const [scrollIntentSegmentId, setScrollIntentSegmentId] = useState<
    string | null
  >(null);
  const displayRowsRef = useRef(displayRows);
  displayRowsRef.current = displayRows;

  // Keep following while a live row is still painting, even if status already
  // flipped idle — covers the last interim→commit handoff after stop.
  const stickFollowContent = followContent || live != null;

  const {
    isPinned,
    newSinceUnpinned,
    jumpToBottom,
    releasePin,
    beginProgrammaticScroll,
  } = useStickToBottomScroll(
      scrollRef,
      {
        historyLength: committed.length,
        hasLiveRow: live != null,
      },
      {
        enabled: !isEmpty,
        watchCharacterData: false,
        followContent: stickFollowContent,
        // Virtual list sticks via scrollToIndex — DOM scrollHeight overshoots
        // when row estimates inflate getTotalSize().
        applyScroll: false,
      },
    );

  const clearCitationFocus = useCallback(() => {
    setFocusedSegmentId(null);
    setScrollIntentSegmentId(null);
  }, []);

  const onScrollIntentConsumed = useCallback(() => {
    setScrollIntentSegmentId(null);
  }, []);

  const handleJumpToLive = useCallback(() => {
    clearCitationFocus();
    jumpToBottom();
  }, [clearCitationFocus, jumpToBottom]);

  // Citation chip → this column (direction-scoped; or own segment).
  useEffect(() => {
    return listenTranscriptFocus((detail) => {
      const dir =
        detail.direction === "outbound" || detail.direction === "inbound"
          ? detail.direction
          : undefined;
      if (dir && dir !== direction) return;

      const ownsSegment = displayRowsRef.current.some(
        (row) =>
          row.kind === "committed" && row.segment.id === detail.segmentId,
      );
      if (!ownsSegment) return;

      releasePin();
      setFocusedSegmentId(detail.segmentId);
      setScrollIntentSegmentId(detail.segmentId);
    });
  }, [direction, releasePin]);

  // Flash then clear emphasis (keep scroll position / unpinned).
  useEffect(() => {
    if (!focusedSegmentId) return;
    const timer = window.setTimeout(() => {
      setFocusedSegmentId(null);
    }, CITATION_FOCUS_MS);
    return () => window.clearTimeout(timer);
  }, [focusedSegmentId]);

  const loadOlderWithAnchor = useCallback(async () => {
    if (!hasMoreOlder || loadingOlder) return;
    const el = scrollRef.current;
    const oldHeight = el?.scrollHeight ?? 0;
    const oldTop = el?.scrollTop ?? 0;
    await loadOlder();
    requestAnimationFrame(() => {
      if (el) {
        el.scrollTop = oldTop + (el.scrollHeight - oldHeight);
      }
    });
  }, [hasMoreOlder, loadOlder, loadingOlder]);

  const onRangeNearTop = useCallback(() => {
    void loadOlderWithAnchor();
  }, [loadOlderWithAnchor]);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const onScroll = () => {
      if (
        el.scrollTop < LIVE_LOAD_MORE_TOP_THRESHOLD_PX &&
        hasMoreOlder &&
        !loadingOlder
      ) {
        void loadOlderWithAnchor();
      }
    };

    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [hasMoreOlder, loadOlderWithAnchor, loadingOlder]);

  let body: ReactNode;
  if (isEmpty) {
    const kind = idleBadge?.kind;
    if (kind === "setup" || kind === "api-key") {
      body = (
        <EmptyState
          icon={Settings}
          title={idleBadge?.label ?? "Setup"}
          description={idleBadge?.title ?? "Finish setup in Settings to start."}
          action={
            onOpenSettings ? (
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={onOpenSettings}
              >
                Settings
              </Button>
            ) : null
          }
        />
      );
    } else if (kind === "error" || kind === "audio-lost") {
      body = (
        <EmptyState
          icon={direction === "inbound" ? Volume2 : Mic}
          title={idleBadge?.label ?? "Unavailable"}
          description={idleBadge?.title}
        />
      );
    } else {
      body = (
        <div className="flex min-h-[200px] h-full items-center justify-center p-6 text-center">
          <p className="m-0 max-w-[28ch] text-sm text-muted-foreground">
            {placeholder}
          </p>
        </div>
      );
    }
  } else {
    body = (
      <>
        <TranscriptTableHeader
          layout={transcriptLayout}
          variant={transcriptVariant}
        />
        <div className={transcriptListPadClass}>
          <LiveTranscriptVirtualList
            direction={direction}
            displayRows={displayRows}
            rowCount={rowCount}
            liveTailText={liveTailText}
            layout={transcriptLayout}
            variant={transcriptVariant}
            scrollRef={scrollRef}
            isPinned={isPinned}
            followContent={stickFollowContent}
            beginProgrammaticScroll={beginProgrammaticScroll}
            focusedSegmentId={focusedSegmentId}
            scrollIntentSegmentId={scrollIntentSegmentId}
            onScrollIntentConsumed={onScrollIntentConsumed}
            onRangeNearTop={onRangeNearTop}
          />
        </div>
      </>
    );
  }

  return (
    <div
      id={columnId}
      className={cn(
        "flex h-full min-h-0 min-w-0 flex-col overflow-hidden",
        elevated ? pipelineToolbarClasses.paneSurface : "bg-card",
      )}
    >
      {banners || toolbar ? (
        <div
          className={cn(
            "sticky top-0 z-[4] shrink-0 has-[[data-state=open]]:z-[12]",
            pipelineToolbarClasses.paneToolbarRail,
          )}
        >
          {banners}
          {toolbar}
        </div>
      ) : null}
      <div className="relative min-h-0 flex-1 bg-card">
        <div
          className="h-full min-h-0 overflow-y-auto p-0"
          ref={scrollRef}
          aria-live="polite"
          aria-relevant="additions text"
        >
          {body}
        </div>
        {!isPinned && !isEmpty ? (
          <Button
            type="button"
            size="sm"
            variant="secondary"
            className={cn(
              "absolute bottom-3 left-1/2 z-[3] -translate-x-1/2 shadow-md",
              "border border-border bg-card/95 backdrop-blur-sm",
            )}
            onClick={handleJumpToLive}
          >
            <ArrowDown className="size-3.5" aria-hidden strokeWidth={2} />
            Jump to live
            {newSinceUnpinned > 0 ? (
              <Badge
                variant="secondary"
                className="h-5 min-w-5 border-transparent bg-primary/20 px-1.5 text-primary"
                aria-label={`${newSinceUnpinned} new lines`}
              >
                {newSinceUnpinned > 9 ? "9+" : newSinceUnpinned}
              </Badge>
            ) : null}
          </Button>
        ) : null}
      </div>
    </div>
  );
}

export const LiveTranscriptColumn = memo(LiveTranscriptColumnInner);
