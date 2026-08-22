import { describe, expect, it } from "vitest";
import {
  getAudioPathMode,
  isColumnAwaitingDirectStandby,
  isColumnPipelineLive,
  isDirectionActive,
  isDirectionDirect,
  isDirectionTranslating,
  isInboundAudioActive,
  isOutboundAudioActive,
  isPipelineBusy,
  getNewMeetingLock,
  needsPlaybackMode,
  normalizeAppStatus,
  normalizePipelineOutputMode,
  normalizePipelineState,
} from "./pipelineStatus";
import { baseConfig, baseStatus } from "@/test/fixtures/config";

describe("normalizePipelineState", () => {
  it("maps legacy idle to off", () => {
    expect(normalizePipelineState("idle")).toBe("off");
    expect(normalizePipelineState("direct")).toBe("direct");
  });
});

describe("normalizeAppStatus", () => {
  it("normalizes pipeline states", () => {
    expect(
      normalizeAppStatus({
        running: false,
        outbound: "idle" as never,
        inbound: "direct",
        error: null,
      }),
    ).toEqual({
      running: false,
      outbound: "off",
      inbound: "direct",
      error: null,
      outboundAudio: "ok",
      inboundAudio: "ok",
      micMuted: false,
      speakerMuted: false,
    });
  });
});

describe("normalizePipelineOutputMode", () => {
  it("maps legacy voice to translated", () => {
    expect(normalizePipelineOutputMode("voice")).toBe("translated");
  });

  it("preserves known modes", () => {
    expect(normalizePipelineOutputMode("originalAudio")).toBe("originalAudio");
    expect(normalizePipelineOutputMode("textOnly")).toBe("textOnly");
  });
});

describe("needsPlaybackMode", () => {
  it("requires playback for translated and original", () => {
    expect(needsPlaybackMode("translated")).toBe(true);
    expect(needsPlaybackMode("originalAudio")).toBe(true);
    expect(needsPlaybackMode("textOnly")).toBe(false);
  });
});

describe("isDirectionTranslating", () => {
  it("is true for starting, stopping, or active", () => {
    expect(
      isDirectionTranslating({ ...baseStatus, outbound: "starting" }, "outbound"),
    ).toBe(true);
    expect(
      isDirectionTranslating({ ...baseStatus, outbound: "stopping" }, "outbound"),
    ).toBe(true);
    expect(
      isDirectionTranslating({ ...baseStatus, inbound: "active" }, "inbound"),
    ).toBe(true);
    expect(
      isDirectionTranslating({ ...baseStatus, outbound: "direct" }, "outbound"),
    ).toBe(false);
  });

  it("returns false when status is null", () => {
    expect(isDirectionTranslating(null, "outbound")).toBe(false);
  });
});

describe("isDirectionDirect", () => {
  it("detects direct mode per column", () => {
    expect(
      isDirectionDirect({ ...baseStatus, outbound: "direct" }, "outbound"),
    ).toBe(true);
    expect(
      isDirectionDirect({ ...baseStatus, inbound: "direct" }, "inbound"),
    ).toBe(true);
    expect(
      isDirectionDirect({ ...baseStatus, outbound: "active" }, "outbound"),
    ).toBe(false);
  });
});

describe("getAudioPathMode", () => {
  it("maps pipeline state to direct or translate", () => {
    expect(
      getAudioPathMode({ ...baseStatus, outbound: "active" }, "outbound"),
    ).toBe("translate");
    expect(
      getAudioPathMode({ ...baseStatus, inbound: "direct" }, "inbound"),
    ).toBe("direct");
  });
});

describe("isPipelineBusy", () => {
  it("is true when either column is starting, stopping, or active", () => {
    expect(
      isPipelineBusy({ ...baseStatus, outbound: "starting" }),
    ).toBe(true);
    expect(
      isPipelineBusy({ ...baseStatus, outbound: "stopping" }),
    ).toBe(true);
    expect(
      isPipelineBusy({ ...baseStatus, inbound: "active" }),
    ).toBe(true);
    expect(isPipelineBusy(baseStatus)).toBe(false);
    expect(
      isPipelineBusy({ ...baseStatus, outbound: "direct", inbound: "direct" }),
    ).toBe(false);
  });
});

describe("getNewMeetingLock", () => {
  it("locks while either column is starting", () => {
    expect(
      getNewMeetingLock(
        { ...baseStatus, outbound: "starting" },
        baseConfig,
        true,
        true,
      ).locked,
    ).toBe(true);
    expect(
      getNewMeetingLock(
        { ...baseStatus, inbound: "active", outbound: "direct" },
        baseConfig,
        true,
        true,
      ).locked,
    ).toBe(false);
  });

  it("locks while either column is stopping", () => {
    expect(
      getNewMeetingLock(
        { ...baseStatus, outbound: "stopping" },
        baseConfig,
        true,
        true,
      ),
    ).toEqual({
      locked: true,
      hint: "Wait until translation finishes stopping",
    });
  });

  it("locks while direct standby is pending", () => {
    expect(
      getNewMeetingLock(baseStatus, baseConfig, true, true).locked,
    ).toBe(true);
  });

  it("unlocks when pipeline is idle without standby", () => {
    expect(
      getNewMeetingLock(
        baseStatus,
        { ...baseConfig, keepDirectAudio: false },
        true,
        true,
      ).locked,
    ).toBe(false);
  });
});

describe("isOutboundAudioActive", () => {
  it("includes direct, starting, and active", () => {
    expect(
      isOutboundAudioActive({ ...baseStatus, outbound: "direct" }),
    ).toBe(true);
    expect(
      isOutboundAudioActive({ ...baseStatus, outbound: "starting" }),
    ).toBe(true);
    expect(isOutboundAudioActive(baseStatus)).toBe(false);
  });
});

describe("isInboundAudioActive", () => {
  it("includes direct, starting, and active", () => {
    expect(
      isInboundAudioActive({ ...baseStatus, inbound: "direct" }),
    ).toBe(true);
    expect(isInboundAudioActive(baseStatus)).toBe(false);
  });
});

describe("isColumnPipelineLive", () => {
  it("delegates to outbound and inbound helpers", () => {
    expect(
      isColumnPipelineLive({ ...baseStatus, outbound: "direct" }, "outbound"),
    ).toBe(true);
    expect(
      isColumnPipelineLive({ ...baseStatus, inbound: "direct" }, "inbound"),
    ).toBe(true);
    expect(isColumnPipelineLive(baseStatus, "outbound")).toBe(false);
  });
});

describe("isColumnAwaitingDirectStandby", () => {
  it("is true when direct standby is pending", () => {
    expect(
      isColumnAwaitingDirectStandby(baseConfig, baseStatus, "outbound", true),
    ).toBe(true);
  });

  it("is false when pipeline is already live", () => {
    expect(
      isColumnAwaitingDirectStandby(
        baseConfig,
        { ...baseStatus, outbound: "direct" },
        "outbound",
        true,
      ),
    ).toBe(false);
  });

  it("is false when keepDirectAudio is off", () => {
    expect(
      isColumnAwaitingDirectStandby(
        { ...baseConfig, keepDirectAudio: false },
        baseStatus,
        "outbound",
        true,
      ),
    ).toBe(false);
  });
});

describe("isDirectionActive", () => {
  it("is true only for active state", () => {
    expect(
      isDirectionActive({ ...baseStatus, outbound: "active" }, "outbound"),
    ).toBe(true);
    expect(
      isDirectionActive({ ...baseStatus, outbound: "starting" }, "outbound"),
    ).toBe(false);
  });
});
