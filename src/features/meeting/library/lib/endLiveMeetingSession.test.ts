import { describe, expect, it, vi } from "vitest";

import { endLiveMeetingSession } from "./endLiveMeetingSession";

describe("endLiveMeetingSession", () => {
  it("ends the meeting even when stopping audio paths fails (device lost)", async () => {
    const endMeeting = vi.fn().mockResolvedValue({ id: "m1", status: "ended" });
    const stopOutbound = vi
      .fn()
      .mockRejectedValue(new Error("Playback device unavailable"));
    const stopInbound = vi
      .fn()
      .mockRejectedValue(new Error("Capture device unavailable"));

    await endLiveMeetingSession({
      meetingId: "m1",
      stopOutbound,
      stopInbound,
      endMeeting,
    });

    expect(stopOutbound).toHaveBeenCalledOnce();
    expect(stopInbound).toHaveBeenCalledOnce();
    expect(endMeeting).toHaveBeenCalledWith("m1");
  });

  it("still ends when only one audio path fails", async () => {
    const endMeeting = vi.fn().mockResolvedValue({});
    await endLiveMeetingSession({
      meetingId: "m2",
      stopOutbound: vi.fn().mockResolvedValue(undefined),
      stopInbound: vi.fn().mockRejectedValue(new Error("device gone")),
      endMeeting,
    });
    expect(endMeeting).toHaveBeenCalledWith("m2");
  });

  it("propagates endMeeting failures", async () => {
    await expect(
      endLiveMeetingSession({
        meetingId: "m3",
        stopOutbound: vi.fn().mockResolvedValue(undefined),
        stopInbound: vi.fn().mockResolvedValue(undefined),
        endMeeting: vi.fn().mockRejectedValue(new Error("db locked")),
      }),
    ).rejects.toThrow("db locked");
  });
});
