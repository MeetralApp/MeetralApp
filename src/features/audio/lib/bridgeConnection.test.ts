import { describe, expect, it } from "vitest";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import {
  canShowReconnectedBanner,
  columnLabel,
  formatReconnectAttemptSuffix,
  getBridgeState,
  RECONNECTED_COOLDOWN_MS,
} from "./bridgeConnection";

const baseStatus: AppStatus = {
  running: true,
  outbound: "active",
  inbound: "off",
  error: null,
};

describe("getBridgeState", () => {
  it("reads per-direction bridge fields", () => {
    const status: AppStatus = {
      ...baseStatus,
      outboundBridge: "reconnecting",
      inboundBridge: "ready",
    };
    expect(getBridgeState(status, "outbound")).toBe("reconnecting");
    expect(getBridgeState(status, "inbound")).toBe("ready");
  });

  it("defaults to idle when missing", () => {
    expect(getBridgeState(baseStatus, "outbound")).toBe("idle");
  });
});

describe("columnLabel", () => {
  it("maps direction to column title", () => {
    expect(columnLabel("outbound")).toBe("You");
    expect(columnLabel("inbound")).toBe("Meeting");
  });
});

describe("canShowReconnectedBanner", () => {
  it("respects cooldown window", () => {
    const now = 10_000;
    expect(canShowReconnectedBanner("outbound", now)).toBe(true);
    writeCooldown("outbound", now);
    expect(canShowReconnectedBanner("outbound", now + 1_000)).toBe(false);
    expect(
      canShowReconnectedBanner("outbound", now + RECONNECTED_COOLDOWN_MS),
    ).toBe(true);
  });
});

describe("formatReconnectAttemptSuffix", () => {
  it("shows capped attempts during quick retry", () => {
    expect(formatReconnectAttemptSuffix(2)).toBe(" (attempt 2/5)");
  });

  it("shows open-ended retry after quick phase", () => {
    expect(formatReconnectAttemptSuffix(6)).toBe(" (attempt 6, retrying…)");
  });
});

function writeCooldown(direction: "outbound" | "inbound", ts: number) {
  sessionStorage.setItem(`reconnected-last-${direction}`, String(ts));
}
