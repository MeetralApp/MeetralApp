import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MeetingsProvider } from "@/features/meeting/library/context/MeetingsProvider";
import { useMeetings } from "@/features/meeting/library/context/useMeetings";

vi.mock("../hooks/useMeetingChanged", () => ({
  useMeetingChanged: () => {},
}));

vi.mock("../lib/meetingApi", () => ({
  listMeetingFolders: vi.fn().mockResolvedValue([]),
  listMeetings: vi.fn().mockResolvedValue({ meetings: [], total: 0 }),
  createMeetingFolder: vi.fn(),
  renameMeetingFolder: vi.fn(),
  deleteMeetingFolder: vi.fn(),
  reorderMeetingFolders: vi.fn(),
  createMeeting: vi.fn(),
  endMeeting: vi.fn(),
  renameMeeting: vi.fn(),
  moveMeeting: vi.fn(),
  deleteMeeting: vi.fn(),
}));

function Probe() {
  const { loading, folders, meetings } = useMeetings();
  return (
    <div>
      <span data-testid="loading">{loading ? "yes" : "no"}</span>
      <span data-testid="folders">{folders.length}</span>
      <span data-testid="meetings">{meetings.length}</span>
    </div>
  );
}

describe("MeetingsContext", () => {
  it("provides meetings state after refresh", async () => {
    render(
      <MeetingsProvider>
        <Probe />
      </MeetingsProvider>,
    );

    expect((await screen.findByTestId("loading")).textContent).toBe("no");
    expect(screen.getByTestId("folders").textContent).toBe("0");
    expect(screen.getByTestId("meetings").textContent).toBe("0");
  });
});
