import type { SegmentCitation } from "@/features/meeting/library/lib/meetingTypes";
import { segmentSnippetText } from "@/features/meeting/library/lib/segmentSnippet";

/**
* View-resolved citation details — looked up from transcript data the surface
* already displays; never fetched via new IPC. Absent ⇒ timestamp-only chip.
*/
export type CitationDescription = {
  /** Segment text snippet — the chip truncates it inside the tooltip. */
  snippet?: string;
  /** Speaker label ("You" / "Meeting"); defaults to the direction label. */
  speakerLabel?: string;
};

export type DescribeCitation = (
  citation: SegmentCitation,
) => CitationDescription | undefined;

type SnippetSegment = {
  sourceText: string;
  translatedText: string;
};

/**
* Build a `DescribeCitation` over a segmentId → segment map. Snippet prefers
* the translated text (what the user reads in the transcript view).
*/
export function describeCitationFromSegments(
  byId: ReadonlyMap<string, SnippetSegment>,
): DescribeCitation {
  return (citation) => {
    const segment = byId.get(citation.segmentId);
    if (!segment) return undefined;
    const snippet = segmentSnippetText(segment);
    return snippet ? { snippet } : undefined;
  };
}
