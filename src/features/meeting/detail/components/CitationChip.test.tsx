import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { CitationChip } from "./CitationChip";
import { describeCitationFromSegments } from "../lib/citationDescription";

describe("describeCitationFromSegments", () => {
  const byId = new Map([
    ["s1", { sourceText: "raw text", translatedText: "bản dịch" }],
    ["s2", { sourceText: "only raw", translatedText: "" }],
  ]);
  const describeCitation = describeCitationFromSegments(byId);

  it("prefers the translated text for the snippet", () => {
    expect(describeCitation({ segmentId: "s1" })).toEqual({
      snippet: "bản dịch",
    });
  });

  it("falls back to the source text", () => {
    expect(describeCitation({ segmentId: "s2" })).toEqual({
      snippet: "only raw",
    });
  });

  it("returns undefined for unknown segment ids", () => {
    expect(describeCitation({ segmentId: "nope" })).toBeUndefined();
  });
});

describe("CitationChip", () => {
  it("static chip title carries speaker + timestamp (no snippet)", () => {
    render(
      <CitationChip
        citation={{ segmentId: "s1", startedAtMs: 4200, direction: "inbound" }}
      />,
    );
    expect(screen.getByText("0:04").getAttribute("title")).toBe(
      "0:04 · Meeting",
    );
  });

  it("static chip title includes the resolved snippet", () => {
    render(
      <CitationChip
        citation={{ segmentId: "s1", startedAtMs: 4200, direction: "outbound" }}
        describe={() => ({ snippet: "We approved the budget" })}
      />,
    );
    expect(screen.getByText("0:04").getAttribute("title")).toBe(
      "You · 0:04\nWe approved the budget",
    );
  });

  it("clickable chip aria-label carries speaker + timestamp", () => {
    const onClick = vi.fn();
    render(
      <TooltipProvider>
        <CitationChip
          citation={{
            segmentId: "s1",
            startedAtMs: 65000,
            direction: "inbound",
          }}
          describe={() => ({ snippet: "snippet" })}
          onClick={onClick}
        />
      </TooltipProvider>,
    );
    const chip = screen.getByRole("button", { name: "Jump to 1:05 · Meeting" });
    chip.click();
    expect(onClick).toHaveBeenCalledWith({
      segmentId: "s1",
      startedAtMs: 65000,
      direction: "inbound",
    });
  });
});
