import { renderHook, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import { useBridgeConnectionNotice } from "./useBridgeConnectionNotice";
import {
  RECONNECT_DEBOUNCE_MS,
  RECONNECTED_BANNER_MS,
} from "../lib/bridgeConnection";

const baseStatus: AppStatus = {
  running: true,
  outbound: "active",
  inbound: "off",
  error: null,
  outboundBridge: "ready",
};

describe("useBridgeConnectionNotice", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    sessionStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
    sessionStorage.clear();
  });

  it("debounces reconnecting banner", () => {
    const { result, rerender } = renderHook(
      ({ status }) => useBridgeConnectionNotice(status, "outbound"),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: { ...baseStatus, outboundBridge: "reconnecting" },
    });
    expect(result.current.mode).toBeNull();

    act(() => {
      vi.advanceTimersByTime(RECONNECT_DEBOUNCE_MS);
    });
    expect(result.current.mode).toBe("reconnecting");

    rerender({
      status: { ...baseStatus, outboundBridge: "ready" },
    });
    expect(result.current.mode).toBe("reconnected");

    act(() => {
      vi.advanceTimersByTime(RECONNECTED_BANNER_MS);
    });
    expect(result.current.mode).toBeNull();
  });

  it("skips reconnecting banner when recovery is fast", () => {
    const { result, rerender } = renderHook(
      ({ status }) => useBridgeConnectionNotice(status, "outbound"),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: { ...baseStatus, outboundBridge: "reconnecting" },
    });
    act(() => {
      vi.advanceTimersByTime(200);
    });
    rerender({
      status: { ...baseStatus, outboundBridge: "ready" },
    });
    expect(result.current.mode).toBeNull();
  });

  it("calls onLongReconnectRestored after long reconnect", () => {
    const onLongReconnectRestored = vi.fn();
    const { rerender } = renderHook(
      ({ status }) =>
        useBridgeConnectionNotice(status, "outbound", {
          onLongReconnectRestored,
        }),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: { ...baseStatus, outboundBridge: "reconnecting" },
    });
    act(() => {
      vi.advanceTimersByTime(RECONNECT_DEBOUNCE_MS);
    });
    act(() => {
      vi.advanceTimersByTime(3_500);
    });
    rerender({
      status: { ...baseStatus, outboundBridge: "ready" },
    });
    expect(onLongReconnectRestored).toHaveBeenCalledWith("You");
  });
});
