import { Node, mergeAttributes } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
import { formatSegmentTimestamp } from "../timelineTranscript";
import { anchorColumnLabel } from "../summaryDisplay";
import { CitationNodeView } from "./CitationNodeView";
import { cn } from "@/shared/lib/utils";
import {
  ANCHOR_TIMER_BADGE_CLASS,
  ANCHOR_TIMER_BADGE_INTERACTIVE_CLASS,
  anchorBadgeFullTitle,
  anchorBadgeInlineText,
  anchorDirectionDotClass,
} from "@/shared/lib/anchorBadgeStyle";

export type CitationAttrs = {
  segmentId: string;
  direction: string;
  startedAtMs: number;
  /** Transcript snippet captured at insert time (chat tooltip parity). */
  snippet?: string;
};

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    citation: {
      insertCitation: (attrs: CitationAttrs) => ReturnType;
    };
  }
}

export function citationLabel(attrs: {
  direction: string;
  startedAtMs: number;
}): string {
  return anchorBadgeInlineText(
    formatSegmentTimestamp(attrs.startedAtMs),
    anchorColumnLabel(attrs.direction),
  );
}

export type CitationExtensionOptions = {
  onCitationClick?: (attrs: CitationAttrs) => void;
  HTMLAttributes: Record<string, unknown>;
};

// Editor chrome wraps the shared anchor-badge recipe (design-system/MASTER.md); the
// direction dot carries speaker semantics, so no per-direction border.
const CITATION_CLASS = cn(
  ANCHOR_TIMER_BADGE_CLASS,
  ANCHOR_TIMER_BADGE_INTERACTIVE_CLASS,
  "cursor-pointer select-none whitespace-nowrap align-baseline leading-snug",
  "[&.ProseMirror-selectednode]:outline [&.ProseMirror-selectednode]:outline-2 [&.ProseMirror-selectednode]:outline-offset-1 [&.ProseMirror-selectednode]:outline-ring/55",
);

function citationDomAttrs(
  nodeAttrs: {
    segmentId?: unknown;
    direction?: unknown;
    startedAtMs?: unknown;
    snippet?: unknown;
  },
  HTMLAttributes: Record<string, unknown>,
  optionsHtml: Record<string, unknown> = {},
) {
  const direction = String(nodeAttrs.direction ?? "outbound");
  const snippet =
    typeof nodeAttrs.snippet === "string" && nodeAttrs.snippet
      ? nodeAttrs.snippet
      : undefined;
  return mergeAttributes(optionsHtml, HTMLAttributes, {
    "data-type": "citation",
    "data-segment-id": String(nodeAttrs.segmentId ?? ""),
    "data-direction": direction,
    "data-started-at-ms": String(nodeAttrs.startedAtMs ?? 0),
    ...(snippet ? { "data-snippet": snippet } : {}),
    class: cn(CITATION_CLASS, "mx-0.5"),
    contenteditable: "false",
    // Native tooltip — same speaker · timer (…+ snippet) semantics as the
    // React badge; pre-snippet docs fall back to timer · speaker.
    title: anchorBadgeFullTitle(
      formatSegmentTimestamp(Number(nodeAttrs.startedAtMs ?? 0)),
      anchorColumnLabel(direction),
      snippet,
    ),
  });
}

export const Citation = Node.create<CitationExtensionOptions>({
  name: "citation",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addOptions() {
    return {
      onCitationClick: undefined,
      HTMLAttributes: {},
    };
  },

  addAttributes() {
    return {
      segmentId: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-segment-id") ?? "",
        renderHTML: (attrs) => ({ "data-segment-id": attrs.segmentId }),
      },
      direction: {
        default: "outbound",
        parseHTML: (el) => el.getAttribute("data-direction") ?? "outbound",
        renderHTML: (attrs) => ({ "data-direction": attrs.direction }),
      },
      startedAtMs: {
        default: 0,
        parseHTML: (el) => Number(el.getAttribute("data-started-at-ms") ?? 0),
        renderHTML: (attrs) => ({
          "data-started-at-ms": String(attrs.startedAtMs ?? 0),
        }),
      },
      snippet: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-snippet") ?? "",
        // Older docs (and citations without one) simply omit the attribute.
        renderHTML: (attrs) =>
          attrs.snippet ? { "data-snippet": attrs.snippet } : {},
      },
    };
  },

  parseHTML() {
    return [{ tag: 'span[data-type="citation"]' }];
  },

  renderHTML({ node, HTMLAttributes }) {
    const direction = String(node.attrs.direction ?? "outbound");
    // Pill text mirrors the chat badge: dot + timer only; the speaker lives
    // in the tooltip (native title, see citationDomAttrs).
    const timer = formatSegmentTimestamp(Number(node.attrs.startedAtMs ?? 0));
    return [
      "span",
      citationDomAttrs(node.attrs, HTMLAttributes, this.options.HTMLAttributes),
      [
        "span",
        {
          class: anchorDirectionDotClass(direction),
          "aria-hidden": "true",
        },
      ],
      timer,
    ];
  },

  renderText({ node }) {
    return citationLabel({
      direction: String(node.attrs.direction ?? "outbound"),
      startedAtMs: Number(node.attrs.startedAtMs ?? 0),
    });
  },

  addCommands() {
    return {
      insertCitation:
        (attrs) =>
        ({ commands }) =>
          commands.insertContent({
            type: this.name,
            attrs,
          }),
    };
  },

  addNodeView() {
    // React node view renders the shared AnchorTimeBadge
    // (styled AppTooltip); renderHTML above stays the serialized/export fallback.
    return ReactNodeViewRenderer(CitationNodeView);
  },
});
