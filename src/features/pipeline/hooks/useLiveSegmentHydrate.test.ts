import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

const dispatch = vi.fn();
const listMeetingSegmentsTail = vi.fn();
const getActiveMeeting = vi.fn();

vi.mock("@/features/pipeline/context/transcript/useLiveTranscript", () => ({
  useLiveTranscriptDispatch: () => ({
    dispatch,
    activeMeetingId: mockActiveMeetingId,
  }),
}));

vi.mock("@/features/meeting/library/lib/meetingApi", () => ({
  listMeetingSegmentsTail: (...args: unknown[]) => listMeetingSegmentsTail(...args),
  getActiveMeeting: (...args: unknown[]) => getActiveMeeting(...args),
}));

vi.mock("@/features/meeting/library/hooks/useMeetingChanged", () => ({
  useMeetingChanged: vi.fn(),
}));

let mockActiveMeetingId: string | null = "meeting-a";

import { useLiveSegmentHydrate } from "./useLiveSegmentHydrate";

describe("useLiveSegmentHydrate", () => {
  beforeEach(() => {
    dispatch.mockReset();
    listMeetingSegmentsTail.mockReset();
    getActiveMeeting.mockReset();
    mockActiveMeetingId = "meeting-a";
    listMeetingSegmentsTail.mockResolvedValue({
      segments: [{ id: "s1", meetingId: "meeting-a", direction: "outbound", sequence: 1 }],
      hasMoreOlder: false,
    });
  });

  it("hydrates both directions for active meeting", async () => {
    renderHook(() => useLiveSegmentHydrate());
    await waitFor(() => expect(listMeetingSegmentsTail).toHaveBeenCalled());
    expect(listMeetingSegmentsTail).toHaveBeenCalledWith(
      "meeting-a",
      "outbound",
      expect.any(Number),
    );
    expect(listMeetingSegmentsTail).toHaveBeenCalledWith(
      "meeting-a",
      "inbound",
      expect.any(Number),
    );
    await waitFor(() =>
      expect(dispatch).toHaveBeenCalledWith(
        expect.objectContaining({ type: "HYDRATE", meetingId: "meeting-a" }),
      ),
    );
  });

  it("ignores stale hydrate results after meeting changes", async () => {
    let resolveOutbound!: (value: unknown) => void;
    const outboundPending = new Promise((resolve) => {
      resolveOutbound = resolve;
    });

    listMeetingSegmentsTail.mockImplementation(
      (_meetingId: string, direction: string) => {
        if (direction === "outbound") {
          return outboundPending;
        }
        return Promise.resolve({ segments: [], hasMoreOlder: false });
      },
    );

    const { rerender } = renderHook(() => useLiveSegmentHydrate());
    await waitFor(() => expect(listMeetingSegmentsTail).toHaveBeenCalled());

    mockActiveMeetingId = "meeting-b";
    listMeetingSegmentsTail.mockResolvedValue({
      segments: [{ id: "b1", meetingId: "meeting-b", sequence: 1 }],
      hasMoreOlder: false,
    });
    rerender();

    await waitFor(() =>
      expect(listMeetingSegmentsTail).toHaveBeenCalledWith(
        "meeting-b",
        expect.any(String),
        expect.any(Number),
      ),
    );

    await act(async () => {
      resolveOutbound({
        segments: [{ id: "stale", meetingId: "meeting-a", sequence: 99 }],
        hasMoreOlder: false,
      });
    });

    const hydrateCalls = dispatch.mock.calls.filter(
      (c) => c[0]?.type === "HYDRATE",
    );
    expect(
      hydrateCalls.some((c) => c[0]?.meetingId === "meeting-a" && c[0]?.segments?.[0]?.id === "stale"),
    ).toBe(false);
  });
});
