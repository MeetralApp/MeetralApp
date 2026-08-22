import type { NodeViewProps } from "@tiptap/react";

import { anchorColumnLabel } from "../summaryDisplay";
import { formatSegmentTimestamp } from "../timelineTranscript";
import { AnchorTimeBadge } from "@/shared/components/AnchorTimeBadge";
import { cn } from "@/shared/lib/utils";
import type {
  CitationAttrs,
  CitationExtensionOptions,
} from "./citationExtension";

/**
* React node view for the citation atom: renders the one shared
* `AnchorTimeBadge`, so the in-doc chip (dot + pill + styled AppTooltip with
* speaker · timer + snippet + jump click) matches artifact citations —
* no more native-title pill (design-system/pages/meeting-detail.md). `renderHTML` keeps the static
* pill + native title as the serialized/export fallback only.
*/
export function CitationNodeView({
  node,
  extension,
  selected,
}: NodeViewProps) {
  const attrs = node.attrs as CitationAttrs;
  const direction = attrs.direction || "outbound";
  const timer = formatSegmentTimestamp(Number(attrs.startedAtMs ?? 0));
  const speaker = anchorColumnLabel(direction);
  const onCitationClick = (extension.options as CitationExtensionOptions)
    .onCitationClick;

  return (
    // Swallow mouse events so clicking the chip only jumps (no atom
    // selection), matching the previous plain-DOM node view.
    <span
      className="inline-flex align-baseline"
      onMouseDown={(event) => event.stopPropagation()}
      onClick={(event) => event.stopPropagation()}
    >
      <AnchorTimeBadge
        timer={timer}
        direction={direction}
        speaker={speaker}
        snippet={attrs.snippet || undefined}
        onClick={
          onCitationClick ? () => onCitationClick({ ...attrs }) : undefined
        }
        className={cn(
          "mx-0.5 select-none whitespace-nowrap leading-snug",
          onCitationClick && "cursor-pointer",
          selected && "outline outline-2 outline-offset-1 outline-ring/55",
        )}
      />
    </span>
  );
}
