import { Extension } from "@tiptap/core";
import Suggestion from "@tiptap/suggestion";
import { PluginKey } from "@tiptap/pm/state";
import type { Editor } from "@tiptap/core";

import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  ANCHOR_SNIPPET_CAP,
  segmentSnippetText,
} from "@/features/meeting/library/lib/segmentSnippet";
import {
  formatSegmentTimestamp,
  timelineSpeakerLabel,
} from "../timelineTranscript";
import type { CitationAttrs } from "./citationExtension";
import { createSuggestionPopupRenderer } from "./suggestionPopup";

export type CiteSuggestionItem = {
  segment: TranscriptSegment;
  label: string;
  snippet: string;
};

function buildItems(
  segments: TranscriptSegment[],
  query: string,
): CiteSuggestionItem[] {
  const q = query.trim().toLowerCase();
  const scored = segments.map((segment) => {
    const speaker = timelineSpeakerLabel(segment.direction);
    const timer = formatSegmentTimestamp(segment.startedAtMs);
    const label = `${timer} · ${speaker}`;
    // Translated-first, matching the chat citation tooltip.
    const snippet = segmentSnippetText(segment, ANCHOR_SNIPPET_CAP);
    return { segment, label, snippet };
  });
  const filtered = q
    ? scored.filter(
        (item) =>
          item.label.toLowerCase().includes(q) ||
          item.snippet.toLowerCase().includes(q) ||
          String(item.segment.sequence).includes(q),
      )
    : scored;
  return filtered.slice(0, 8);
}

export function citeItemToAttrs(item: CiteSuggestionItem): CitationAttrs {
  return {
    segmentId: item.segment.id,
    direction: item.segment.direction,
    startedAtMs: item.segment.startedAtMs,
    snippet: item.snippet,
  };
}

function makeSuggestionPlugin(
  editor: Editor,
  char: string,
  pluginKey: PluginKey,
  getSegments: () => TranscriptSegment[],
) {
  return Suggestion<CiteSuggestionItem, CiteSuggestionItem>({
    editor,
    pluginKey,
    char,
    allowSpaces: false,
    startOfLine: false,
    items: ({ query }) => buildItems(getSegments(), query),
    command: ({ editor: ed, range, props }) => {
      const attrs = citeItemToAttrs(props);
      ed.chain()
        .focus()
        .insertContentAt(range, { type: "citation", attrs })
        .run();
    },
    render: createSuggestionPopupRenderer<CiteSuggestionItem>({
      emptyLabel: "No matching segments",
      widthClass: "w-72",
      getRow: (item) => ({
        title: item.label,
        subtitle: item.snippet || "(empty)",
        titleClassName: "tabular-nums",
      }),
    }),
  });
}

/** Suggestion on `@` → pick transcript segment → insert citation atom. */
export const CitationSuggestion = Extension.create<{
  getSegments: () => TranscriptSegment[];
}>({
  name: "citationSuggestion",

  addOptions() {
    return {
      getSegments: () => [],
    };
  },

  addProseMirrorPlugins() {
    const getSegments = () => this.options.getSegments();
    return [
      makeSuggestionPlugin(
        this.editor,
        "@",
        new PluginKey("citationSuggestionAt"),
        getSegments,
      ),
    ];
  },
});
