import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import LibraryMeetingRow from "./LibraryMeetingRow";
import type { MeetingRecord } from "@/features/meeting/library/lib/meetingTypes";
import { TooltipProvider } from "@/shared/ui/tooltip";

function baseMeeting(): MeetingRecord {
  return {
    id: "m1",
    title: "Weekly sync",
    status: "ended",
    folderId: null,
    myLanguage: "en",
    meetingLanguage: "vi",
    startedAtMs: 1_700_000_000_000,
    endedAtMs: 1_700_000_360_000,
    segmentCountOutbound: 1,
    segmentCountInbound: 1,
  };
}

function renderRow(meeting: MeetingRecord) {
  render(
    <TooltipProvider>
      <LibraryMeetingRow meeting={meeting} onSelect={vi.fn()} />
    </TooltipProvider>,
  );
  return screen.getByRole("button");
}

describe("LibraryMeetingRow", () => {
  it("moves Notes/Summary off the title row into the meta line", () => {
    const button = renderRow({
      ...baseMeeting(),
      sessionMode: "notes",
      hasSummary: true,
    });

    expect(button.textContent).toContain("Notes");
    expect(button.textContent).toContain("Summary");
    // No standalone pill badges remain — exact-text match on the title row is gone.
    expect(screen.queryByText("Notes")).toBeNull();
    expect(screen.queryByText("Summary")).toBeNull();
    expect(screen.queryByText("LIVE")).toBeNull();
  });

  it("shows Summary (but not Notes) for interpreter meetings", () => {
    const button = renderRow({ ...baseMeeting(), hasSummary: true });

    expect(button.textContent).toContain("Summary");
    expect(button.textContent).not.toContain("Notes");
  });

  it("keeps LIVE on the title row and hides Summary while live", () => {
    const button = renderRow({
      ...baseMeeting(),
      status: "live",
      sessionMode: "notes",
      hasSummary: true,
    });

    expect(screen.getByText("LIVE")).toBeTruthy();
    // hasSummary is ignored while live; Notes still appears in the meta line.
    expect(button.textContent).toContain("Notes");
    expect(button.textContent).not.toContain("Summary");
  });

  it("shows no content signals when absent", () => {
    const button = renderRow({ ...baseMeeting() });

    expect(button.textContent).not.toContain("Notes");
    expect(button.textContent).not.toContain("Summary");
  });
});
