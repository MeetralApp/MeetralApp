import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { JSONContent } from "@tiptap/core";

import SummaryDocEditor, {
  type SummaryDocEditorHandle,
} from "./SummaryDocEditor";
import type { CitationAttrs } from "../lib/summaryDoc/citationExtension";
import { TooltipProvider } from "@/shared/ui/tooltip";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

const segments: TranscriptSegment[] = [
  {
    id: "seg-in",
    meetingId: "m1",
    direction: "inbound",
    sequence: 2,
    sourceText: "Agreed",
    translatedText: "Đồng ý",
    startedAtMs: 65_000,
    endedAtMs: 66_000,
    connectionGap: false,
  },
];

function docWithCitation(attrs: Record<string, unknown>): JSONContent {
  return {
    type: "doc",
    content: [
      {
        type: "paragraph",
        content: [
          { type: "text", text: "Note " },
          { type: "citation", attrs },
        ],
      },
    ],
  };
}

function citationAttrsOf(doc: JSONContent): Record<string, unknown> {
  let found: Record<string, unknown> | undefined;
  const walk = (node: JSONContent) => {
    if (node.type === "citation") found = (node.attrs ?? {}) as Record<string, unknown>;
    node.content?.forEach(walk);
  };
  walk(doc);
  if (!found) throw new Error("citation atom missing");
  return found;
}

function renderEditor(
  initialDoc: JSONContent,
  onCitationClick?: (attrs: CitationAttrs) => void,
) {
  let handle: SummaryDocEditorHandle | undefined;
  const utils = render(
    <TooltipProvider delayDuration={0}>
      <SummaryDocEditor
        initialDoc={initialDoc}
        segments={segments}
        editable={false}
        onCitationClick={onCitationClick}
        onReady={(h) => {
          handle = h;
        }}
      />
    </TooltipProvider>,
  );
  return { ...utils, getHandle: () => handle };
}

describe("SummaryDocEditor citation chips", () => {
  it("renders the shared badge with jump aria-label and fires onCitationClick", async () => {
    const onCitationClick = vi.fn();
    renderEditor(
      docWithCitation({
        segmentId: "seg-in",
        direction: "inbound",
        startedAtMs: 65_000,
        snippet: "Đồng ý",
      }),
      onCitationClick,
    );
    fireEvent.click(
      await screen.findByRole("button", { name: "Jump to 1:05 · Meeting" }),
    );
    expect(onCitationClick).toHaveBeenCalledWith(
      expect.objectContaining({ segmentId: "seg-in" }),
    );
  });

  it("shows the snippet in the styled tooltip (not native title)", async () => {
    renderEditor(
      docWithCitation({
        segmentId: "seg-in",
        direction: "inbound",
        startedAtMs: 65_000,
        snippet: "Đồng ý",
      }),
      vi.fn(),
    );
    const chip = await screen.findByRole("button", {
      name: "Jump to 1:05 · Meeting",
    });
    expect(chip.getAttribute("title")).toBeNull();
    fireEvent.pointerMove(chip);
    // Radix renders the label twice (visible + visually-hidden a11y mirror
    // for aria-describedby) — findAll avoids the multiple-match throw.
    expect((await screen.findAllByText("Đồng ý")).length).toBeGreaterThan(0);
  });

  // Rust-seeded / legacy docs carry no snippet attr — the editor fills it
  // from the loaded segments so the tooltip shows transcript text.
  it("enriches a snippet-less atom from segments into the doc model", () => {
    const { getHandle } = renderEditor(
      docWithCitation({
        segmentId: "seg-in",
        direction: "inbound",
        startedAtMs: 65_000,
      }),
      vi.fn(),
    );
    const attrs = citationAttrsOf(getHandle()!.getJSON());
    expect(attrs.snippet).toBe("Đồng ý");
  });

  it("keeps an already-captured snippet", () => {
    const { getHandle } = renderEditor(
      docWithCitation({
        segmentId: "seg-in",
        direction: "inbound",
        startedAtMs: 65_000,
        snippet: "Agreed",
      }),
      vi.fn(),
    );
    expect(citationAttrsOf(getHandle()!.getJSON()).snippet).toBe("Agreed");
  });

  it("leaves the atom snippet-less when the segment is gone", () => {
    const { getHandle } = renderEditor(
      docWithCitation({
        segmentId: "missing",
        direction: "inbound",
        startedAtMs: 65_000,
      }),
      vi.fn(),
    );
    // Tiptap round-trips the missing attr through its default ("").
    expect(citationAttrsOf(getHandle()!.getJSON()).snippet).toBeFalsy();
  });
});
