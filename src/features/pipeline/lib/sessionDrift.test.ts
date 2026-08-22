import { beforeEach, describe, expect, it } from "vitest";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import {
  formatSessionElapsed,
  formatSessionElapsedTitle,
  getActiveSince,
  getSessionElapsedMs,
  isDirectionPipelineActive,
  SESSION_DRIFT_THRESHOLD_MS,
  shouldShowSessionDriftHint,
  syncDismissForSession,
  sessionDriftSessionKey,
  readDismissed,
  writeDismissed,
} from "./sessionDrift";
const baseStatus: AppStatus = {
  running: true,
  outbound: "off",
  inbound: "off",
  error: null,
};

describe("getActiveSince", () => {
  it("reads outbound and inbound fields", () => {
    const status: AppStatus = {
      ...baseStatus,
      outboundActiveSince: 1_000,
      inboundActiveSince: 2_000,
    };
    expect(getActiveSince(status, "outbound")).toBe(1_000);
    expect(getActiveSince(status, "inbound")).toBe(2_000);
  });

  it("returns null when status or field is missing", () => {
    expect(getActiveSince(null, "outbound")).toBeNull();
    expect(getActiveSince(baseStatus, "outbound")).toBeNull();
  });
});

describe("formatSessionElapsed", () => {
  it("formats sub-hour durations as m:ss", () => {
    expect(formatSessionElapsed(5_000)).toBe("0:05");
    expect(formatSessionElapsed(125_000)).toBe("2:05");
    expect(formatSessionElapsed(3_599_000)).toBe("59:59");
  });

  it("formats hour-plus durations as h:mm:ss", () => {
    expect(formatSessionElapsed(3_600_000)).toBe("1:00:00");
    expect(formatSessionElapsed(3_905_000)).toBe("1:05:05");
  });
});

describe("getSessionElapsedMs", () => {
  it("never returns negative elapsed time", () => {
    expect(getSessionElapsedMs(1_000, 500)).toBe(0);
    expect(getSessionElapsedMs(1_000, 6_000)).toBe(5_000);
  });
});

describe("formatSessionElapsedTitle", () => {
  it("builds a readable session title", () => {
    expect(formatSessionElapsedTitle(65_000)).toContain("1 minute");
    expect(formatSessionElapsedTitle(3_600_000)).toContain("1 hour");
  });
});

describe("shouldShowSessionDriftHint", () => {
  const since = 1_700_000_000_000;
  const activeOutbound: AppStatus = {
    ...baseStatus,
    outbound: "active",
    outboundActiveSince: since,
  };

  it("returns false when idle", () => {
    expect(
      shouldShowSessionDriftHint({
        status: baseStatus,
        direction: "outbound",
        now: since + SESSION_DRIFT_THRESHOLD_MS,
        dismissed: false,
      }),
    ).toBe(false);
  });

  it("returns false when active but below threshold", () => {
    expect(
      shouldShowSessionDriftHint({
        status: activeOutbound,
        direction: "outbound",
        now: since + SESSION_DRIFT_THRESHOLD_MS - 1,
        dismissed: false,
      }),
    ).toBe(false);
  });

  it("returns true when active at or past threshold", () => {
    expect(
      shouldShowSessionDriftHint({
        status: activeOutbound,
        direction: "outbound",
        now: since + SESSION_DRIFT_THRESHOLD_MS,
        dismissed: false,
      }),
    ).toBe(true);
  });

  it("returns false when dismissed", () => {
    expect(
      shouldShowSessionDriftHint({
        status: activeOutbound,
        direction: "outbound",
        now: since + SESSION_DRIFT_THRESHOLD_MS,
        dismissed: true,
      }),
    ).toBe(false);
  });

  it("uses inbound field for inbound direction", () => {
    const status: AppStatus = {
      ...baseStatus,
      inbound: "active",
      inboundActiveSince: since,
    };
    expect(
      shouldShowSessionDriftHint({
        status,
        direction: "inbound",
        now: since + SESSION_DRIFT_THRESHOLD_MS,
        dismissed: false,
      }),
    ).toBe(true);
    expect(
      shouldShowSessionDriftHint({
        status,
        direction: "outbound",
        now: since + SESSION_DRIFT_THRESHOLD_MS,
        dismissed: false,
      }),
    ).toBe(false);
  });
});

describe("isDirectionPipelineActive", () => {
  it("is true only for active state", () => {
    expect(
      isDirectionPipelineActive(
        { ...baseStatus, outbound: "starting" },
        "outbound",
      ),
    ).toBe(false);
    expect(
      isDirectionPipelineActive(
        { ...baseStatus, inbound: "active" },
        "inbound",
      ),
    ).toBe(true);
    expect(isDirectionPipelineActive(baseStatus, "outbound")).toBe(false);
  });
});

describe("syncDismissForSession", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it("clears dismiss state when session ends", () => {
    writeDismissed("outbound", true);
    sessionStorage.setItem(sessionDriftSessionKey("outbound"), "123");
    syncDismissForSession("outbound", null);
    expect(readDismissed("outbound")).toBe(false);
    expect(sessionStorage.getItem(sessionDriftSessionKey("outbound"))).toBeNull();
  });

  it("resets dismiss when active session id changes", () => {
    writeDismissed("outbound", true);
    sessionStorage.setItem(sessionDriftSessionKey("outbound"), "100");
    syncDismissForSession("outbound", 200);
    expect(readDismissed("outbound")).toBe(false);
    expect(sessionStorage.getItem(sessionDriftSessionKey("outbound"))).toBe("200");
  });

  it("keeps dismiss for the same session id", () => {
    writeDismissed("outbound", true);
    sessionStorage.setItem(sessionDriftSessionKey("outbound"), "100");
    syncDismissForSession("outbound", 100);
    expect(readDismissed("outbound")).toBe(true);
  });
});
