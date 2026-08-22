import type { JSONContent } from "@tiptap/core";

import {
  SUMMARY_BLOCKS,
  blockLabel,
  blockPoints,
} from "../summaryDisplay";
import type { MeetingBriefSummary } from "../summaryTypes";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  ANCHOR_SNIPPET_CAP,
  segmentSnippetText,
} from "@/features/meeting/library/lib/segmentSnippet";

function segmentForRef(
  segments: TranscriptSegment[],
  direction: string,
  sequence: number,
): TranscriptSegment | undefined {
  return segments.find(
    (s) => s.direction === direction && s.sequence === sequence,
  );
}

function citationNode(
  segment: TranscriptSegment,
): JSONContent {
  return {
    type: "citation",
    attrs: {
      segmentId: segment.id,
      direction: segment.direction,
      startedAtMs: segment.startedAtMs,
      // Seeded atoms carry the same snippet as "@"-inserted ones.
      snippet: segmentSnippetText(segment, ANCHOR_SNIPPET_CAP),
    },
  };
}

function pointParagraph(
  text: string,
  cites: JSONContent[],
  meta?: string,
): JSONContent {
  const content: JSONContent[] = [];
  const trimmed = text.trim();
  if (trimmed) {
    content.push({ type: "text", text: trimmed });
  }
  if (meta) {
    if (content.length) content.push({ type: "text", text: ` (${meta})` });
    else content.push({ type: "text", text: meta });
  }
  for (const cite of cites) {
    if (content.length) content.push({ type: "text", text: " " });
    content.push(cite);
  }
  return {
    type: "paragraph",
    content: content.length ? content : undefined,
  };
}

/** Seed a TipTap doc from structured meeting brief + transcript segments. */
export function seedFromBrief(
  summary: MeetingBriefSummary,
  segments: TranscriptSegment[],
  summaryLanguage: string,
): JSONContent {
  const content: JSONContent[] = [];

  for (const blockKind of SUMMARY_BLOCKS) {
    const points = blockPoints(summary, blockKind);
    if (points.length === 0) continue;

    content.push({
      type: "heading",
      attrs: { level: 3 },
      content: [{ type: "text", text: blockLabel(blockKind, summaryLanguage) }],
    });

    const items: JSONContent[] = [];
    for (const point of points) {
      const cites: JSONContent[] = [];
      const seen = new Set<string>();
      for (const ref of point.segmentRefs ?? []) {
        const seg = segmentForRef(segments, ref.direction, ref.sequence);
        if (!seg || seen.has(seg.id)) continue;
        seen.add(seg.id);
        cites.push(citationNode(seg));
      }
      const metaParts: string[] = [];
      if (blockKind === "actionItems") {
        if (point.owner?.trim()) metaParts.push(`Owner: ${point.owner.trim()}`);
        if (point.due?.trim()) metaParts.push(`Due: ${point.due.trim()}`);
      }
      items.push({
        type: "listItem",
        content: [
          pointParagraph(point.text, cites, metaParts.join(" · ") || undefined),
        ],
      });
    }
    content.push({ type: "bulletList", content: items });
  }

  if (content.length === 0) {
    return {
      type: "doc",
      content: [{ type: "paragraph" }],
    };
  }

  return { type: "doc", content };
}
