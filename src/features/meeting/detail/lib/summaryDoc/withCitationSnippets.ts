import type { JSONContent } from "@tiptap/core";

import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  ANCHOR_SNIPPET_CAP,
  segmentSnippetText,
} from "@/features/meeting/library/lib/segmentSnippet";

/**
* Fill missing citation-atom snippets from the loaded transcript. Rust-seeded
* docs and docs saved before the `snippet` attr existed carry none — this
* resolves them at display time so citation tooltips have a snippet. Atoms
* that already captured a snippet (or whose segment is gone) are untouched.
* Returns the original doc when there is nothing to fill.
*/
export function withCitationSnippets(
  doc: JSONContent,
  segments: TranscriptSegment[],
): JSONContent {
  if (!segments.length) return doc;
  const byId = new Map(segments.map((segment) => [segment.id, segment]));

  // Identity-preserving walk: untouched subtrees keep their reference, so a
  // doc with nothing to fill comes back === the input (memo-friendly).
  const walk = (node: JSONContent): JSONContent => {
    let next = node;
    if (node.content) {
      const content = node.content.map(walk);
      if (content.some((child, index) => child !== node.content![index])) {
        next = { ...next, content };
      }
    }
    if (next.type === "citation") {
      const attrs = (next.attrs ?? {}) as Record<string, unknown>;
      const segmentId = typeof attrs.segmentId === "string" ? attrs.segmentId : "";
      const snippet = typeof attrs.snippet === "string" ? attrs.snippet : "";
      const segment = segmentId ? byId.get(segmentId) : undefined;
      if (segment && !snippet) {
        const resolved = segmentSnippetText(segment, ANCHOR_SNIPPET_CAP);
        if (resolved) {
          next = { ...next, attrs: { ...attrs, snippet: resolved } };
        }
      }
    }
    return next;
  };

  return walk(doc);
}
