import { describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import * as meetingApi from "@/features/meeting/library/lib/meetingApi";
import { useLiveSegmentPagination } from "./useLiveSegmentPagination";

vi.mock("@/features/pipeline/context/transcript/useLiveTranscript", () => ({
  useLiveCommitted: () => [
    {
      id: "s5",
      meetingId: "m1",
      direction: "outbound",
      sequence: 5,
      sourceText: "a",
      translatedText: "b",
      startedAtMs: 0,
      endedAtMs: 0,
      connectionGap: false,
    },
  ],
  useLiveHasMoreOlder: () => true,
  useLiveLoadingOlder: () => false,
  useLiveTranscriptDispatch: () => ({
    activeMeetingId: "m1",
    dispatch: vi.fn(),
  }),
}));

describe("useLiveSegmentPagination", () => {
  it("calls listMeetingSegmentsBefore with min sequence", async () => {
    const spy = vi.spyOn(meetingApi, "listMeetingSegmentsBefore").mockResolvedValue({
      segments: [],
      total: 0,
      hasMoreOlder: false,
    });

    const { result } = renderHook(() => useLiveSegmentPagination("outbound"));
    await result.current.loadOlder();

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith("m1", "outbound", 5, 100);
    });
  });
});
