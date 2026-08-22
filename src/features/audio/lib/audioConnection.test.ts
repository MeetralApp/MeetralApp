import { beforeEach, describe, expect, it } from "vitest";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import {
  formatAudioReconnectAttemptSuffix,
  getAudioState,
  isAnyColumnAudioFault,
  isColumnAudioFault,
  areAllAudioPathsOk,
  canShowAudioReconnectedBanner,
  readAudioReconnectedCooldown,
  writeAudioReconnectedCooldown,
  AUDIO_RECONNECTED_COOLDOWN_MS,
  MAX_AUDIO_RECONNECT_ATTEMPTS,
} from "./audioConnection";

const baseStatus: AppStatus = {
  running: true,
  outbound: "direct",
  inbound: "off",
  error: null,
};

describe("getAudioState", () => {
  it("returns reconnecting when outbound audio is reconnecting", () => {
    const status: AppStatus = {
      ...baseStatus,
      outboundAudio: "reconnecting",
    };
    expect(getAudioState(status, "outbound")).toBe("reconnecting");
  });

  it("returns reconnecting when pipeline is off during direct restart", () => {
    const status: AppStatus = {
      ...baseStatus,
      outbound: "off",
      outboundAudio: "reconnecting",
    };
    expect(getAudioState(status, "outbound")).toBe("reconnecting");
  });

  it("returns lost when translate pipeline errored", () => {
    const status: AppStatus = {
      ...baseStatus,
      outbound: "error",
      outboundAudio: "lost",
    };
    expect(getAudioState(status, "outbound")).toBe("lost");
  });
});

describe("isColumnAudioFault", () => {
  it("is true for reconnecting or lost", () => {
    expect(
      isColumnAudioFault(
        { ...baseStatus, outboundAudio: "reconnecting" },
        "outbound",
      ),
    ).toBe(true);
    expect(
      isColumnAudioFault(
        { ...baseStatus, inboundAudio: "lost" },
        "inbound",
      ),
    ).toBe(true);
    expect(
      isColumnAudioFault({ ...baseStatus, outboundAudio: "ok" }, "outbound"),
    ).toBe(false);
  });
});

describe("isAnyColumnAudioFault", () => {
  it("is true when either column has a fault", () => {
    expect(
      isAnyColumnAudioFault({ ...baseStatus, inboundAudio: "reconnecting" }),
    ).toBe(true);
    expect(isAnyColumnAudioFault(baseStatus)).toBe(false);
  });
});

describe("areAllAudioPathsOk", () => {
  it("requires both columns ok", () => {
    expect(areAllAudioPathsOk(baseStatus)).toBe(true);
    expect(
      areAllAudioPathsOk({ ...baseStatus, inboundAudio: "reconnecting" }),
    ).toBe(false);
  });
});

describe("formatAudioReconnectAttemptSuffix", () => {
  it("formats attempt within max", () => {
    expect(formatAudioReconnectAttemptSuffix(2)).toBe(
      ` (attempt 2/${MAX_AUDIO_RECONNECT_ATTEMPTS})`,
    );
  });

  it("formats open-ended retry after max attempts", () => {
    expect(formatAudioReconnectAttemptSuffix(6)).toBe(
      " (attempt 6, retrying…)",
    );
  });
});

describe("canShowAudioReconnectedBanner", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it("allows banner when no prior cooldown", () => {
    expect(canShowAudioReconnectedBanner("outbound", 10_000)).toBe(true);
  });

  it("blocks banner during cooldown window", () => {
    writeAudioReconnectedCooldown("outbound");
    const now = Date.now();
    expect(canShowAudioReconnectedBanner("outbound", now + 1_000)).toBe(false);
    expect(
      canShowAudioReconnectedBanner(
        "outbound",
        now + AUDIO_RECONNECTED_COOLDOWN_MS,
      ),
    ).toBe(true);
  });

  it("reads and writes cooldown timestamp", () => {
    expect(readAudioReconnectedCooldown("inbound")).toBeNull();
    writeAudioReconnectedCooldown("inbound");
    expect(readAudioReconnectedCooldown("inbound")).not.toBeNull();
  });
});
