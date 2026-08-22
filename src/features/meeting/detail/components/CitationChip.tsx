import { AnchorTimeBadge } from "@/shared/components/AnchorTimeBadge";
import type { SegmentCitation } from "@/features/meeting/library/lib/meetingTypes";
import { anchorColumnLabel } from "@/features/meeting/detail/lib/summaryDisplay";
import { formatSegmentTimestamp } from "@/features/meeting/detail/lib/timelineTranscript";
import type { DescribeCitation } from "../lib/citationDescription";

type CitationChipProps = {
  citation: SegmentCitation;
  /**
  * Resolve snippet/speaker from the surface's loaded transcript.
  * Absent or returning undefined ⇒ timestamp-only chip.
  */
  describe?: DescribeCitation;
  /** When absent the chip renders as a static pill. */
  onClick?: (citation: SegmentCitation) => void;
  className?: string;
};

/**
* Citation chip — SegmentCitation adapter over the shared `AnchorTimeBadge`:
* resolves speaker/snippet, formats the timestamp, forwards the jump click.
*/
export function CitationChip({
  citation,
  describe,
  onClick,
  className,
}: CitationChipProps) {
  const column = anchorColumnLabel(citation.direction ?? "inbound");
  const description = describe?.(citation);
  const speaker = description?.speakerLabel ?? column;
  const timer =
    citation.startedAtMs != null
      ? formatSegmentTimestamp(citation.startedAtMs)
      : "—";
  return (
    <AnchorTimeBadge
      timer={timer}
      direction={citation.direction}
      speaker={speaker}
      snippet={description?.snippet}
      onClick={onClick ? () => onClick(citation) : undefined}
      className={className}
    />
  );
}
