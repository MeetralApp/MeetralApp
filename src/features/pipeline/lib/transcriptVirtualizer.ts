import type { TranscriptLayout } from "@/shared/lib/types/pipeline";
import type { LiveDisplayRow } from "./transcriptView";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

export interface TranscriptScrollIntent {
  segmentId: string;
  direction: "outbound" | "inbound";
}

export type TranscriptEstimateVariant = "bilingual" | "notes";

function baseEstimate(
  segment: TranscriptSegment | undefined,
  variant: TranscriptEstimateVariant = "bilingual",
): number {
  if (!segment) return variant === "notes" ? 56 : 72;

  const chars =
    variant === "notes"
      ? (segment.sourceText?.trim()
          ? segment.sourceText.length
          : (segment.translatedText?.length ?? 0))
      : Math.max(
          segment.sourceText?.length ?? 0,
          segment.translatedText?.length ?? 0,
        );
  const colWidth = variant === "notes" ? 45 : 38;
  const lines = Math.max(1, Math.ceil(chars / colWidth));
  const lineHeight = 26;
  const padding = 32;
  return Math.max(variant === "notes" ? 56 : 72, padding + lines * lineHeight);
}

function pseudoSegmentFromLiveRow(row: Extract<LiveDisplayRow, { kind: "live" }>): TranscriptSegment {
  return {
    id: "live",
    meetingId: "",
    direction: "outbound",
    sequence: row.sequence,
    sourceText: row.block.source ?? "",
    translatedText: row.block.translated ?? "",
    startedAtMs: 0,
    endedAtMs: 0,
    connectionGap: row.block.connectionGap ?? false,
  };
}

export function estimateLiveDisplayRowHeight(
  row: LiveDisplayRow | undefined,
  layout: TranscriptLayout = "sideBySide",
  variant: TranscriptEstimateVariant = "bilingual",
): number {
  if (!row) return variant === "notes" ? 56 : 72;
  if (row.kind === "committed") {
    return estimateTranscriptRowHeight(row.segment, layout, variant);
  }
  return estimateTranscriptRowHeight(
    pseudoSegmentFromLiveRow(row),
    layout,
    variant,
  );
}

/** Rough row height for @tanstack/react-virtual before DOM measure. */
export function estimateTranscriptRowHeight(
  segment: TranscriptSegment | undefined,
  layout: TranscriptLayout = "sideBySide",
  variant: TranscriptEstimateVariant = "bilingual",
): number {
  if (variant === "notes") {
    return baseEstimate(segment, "notes");
  }
  const base = baseEstimate(segment, "bilingual");
  if (layout === "stacked") {
    const sourceChars = segment?.sourceText?.length ?? 0;
    const translatedChars = segment?.translatedText?.length ?? 0;
    const sourceLines = Math.max(1, Math.ceil(sourceChars / 45));
    const translatedLines = Math.max(1, Math.ceil(translatedChars / 45));
    const stacked =
      32 + translatedLines * 26 + 20 + sourceLines * 26 + 24;
    return Math.max(base, stacked);
  }
  return base;
}

/** Cumulative start offset per row index from height estimates. */
export function buildTranscriptOffsets(
  segments: TranscriptSegment[],
  layout: TranscriptLayout,
  variant: TranscriptEstimateVariant = "bilingual",
): number[] {
  const offsets: number[] = [];
  let offset = 0;
  for (const segment of segments) {
    offsets.push(offset);
    offset += estimateTranscriptRowHeight(segment, layout, variant);
  }
  return offsets;
}

export function estimatedTotalTranscriptHeight(
  segments: TranscriptSegment[],
  layout: TranscriptLayout,
  variant: TranscriptEstimateVariant = "bilingual",
): number {
  if (segments.length === 0) return 0;
  const offsets = buildTranscriptOffsets(segments, layout, variant);
  const lastIndex = segments.length - 1;
  return (
    (offsets[lastIndex] ?? 0) +
    estimateTranscriptRowHeight(segments[lastIndex], layout, variant)
  );
}

/** Scroll offset to bring `index` into view using precomputed estimates. */
export function estimatedScrollOffsetForIndex(
  offsets: number[],
  segments: TranscriptSegment[],
  index: number,
  layout: TranscriptLayout,
  viewportHeight: number,
  align: "start" | "center" | "end" = "center",
  variant: TranscriptEstimateVariant = "bilingual",
): number {
  if (index < 0 || index >= segments.length || viewportHeight <= 0) return 0;

  const itemOffset = offsets[index] ?? 0;
  const itemSize = estimateTranscriptRowHeight(segments[index], layout, variant);
  const totalSize = estimatedTotalTranscriptHeight(segments, layout, variant);

  let scrollOffset: number;
  switch (align) {
    case "start":
      scrollOffset = itemOffset;
      break;
    case "end":
      scrollOffset = itemOffset - viewportHeight + itemSize;
      break;
    default:
      scrollOffset = itemOffset - (viewportHeight - itemSize) / 2;
  }

  const maxScroll = Math.max(0, totalSize - viewportHeight);
  return Math.min(maxScroll, Math.max(0, scrollOffset));
}

/** Run `scroll` once the scroll container has a non-zero height (e.g. after tab show). */
export function scrollWhenReady(
  element: HTMLElement | null,
  scroll: () => void,
  maxAttempts = 12,
): () => void {
  let attempts = 0;
  let frame = 0;
  const tick = () => {
    if (element && element.clientHeight > 0) {
      scroll();
      return;
    }
    if (++attempts < maxAttempts) {
      frame = requestAnimationFrame(tick);
    }
  };
  frame = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(frame);
}
