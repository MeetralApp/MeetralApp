import { renderHook, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import { useAudioDeviceNotice } from "./useAudioDeviceNotice";
import {
  AUDIO_RECONNECT_DEBOUNCE_MS,
  AUDIO_RECONNECTED_BANNER_MS,
} from "../lib/audioConnection";

const baseStatus: AppStatus = {
  running: true,
  outbound: "direct",
  inbound: "off",
  error: null,
  outboundAudio: "ok",
};

describe("useAudioDeviceNotice", () => {
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
      ({ status }) => useAudioDeviceNotice(status, "outbound"),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: { ...baseStatus, outboundAudio: "reconnecting" },
    });
    expect(result.current.mode).toBeNull();

    act(() => {
      vi.advanceTimersByTime(AUDIO_RECONNECT_DEBOUNCE_MS);
    });
    expect(result.current.mode).toBe("reconnecting");

    rerender({
      status: { ...baseStatus, outboundAudio: "ok" },
    });
    expect(result.current.mode).toBe("reconnected");

    act(() => {
      vi.advanceTimersByTime(AUDIO_RECONNECTED_BANNER_MS);
    });
    expect(result.current.mode).toBeNull();
  });

  it("skips reconnecting banner when recovery is fast", () => {
    const { result, rerender } = renderHook(
      ({ status }) => useAudioDeviceNotice(status, "outbound"),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: { ...baseStatus, outboundAudio: "reconnecting" },
    });
    act(() => {
      vi.advanceTimersByTime(200);
    });
    rerender({
      status: { ...baseStatus, outboundAudio: "ok" },
    });
    expect(result.current.mode).toBeNull();
  });

  it("does not show reconnected after fatal when pipeline stops", () => {
    const { result, rerender } = renderHook(
      ({ status }) => useAudioDeviceNotice(status, "outbound"),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: {
        ...baseStatus,
        outbound: "active",
        outboundAudio: "reconnecting",
      },
    });
    act(() => {
      vi.advanceTimersByTime(AUDIO_RECONNECT_DEBOUNCE_MS);
    });
    expect(result.current.mode).toBe("reconnecting");

    rerender({
      status: {
        ...baseStatus,
        outbound: "error",
        outboundAudio: "ok",
      },
    });
    expect(result.current.mode).toBeNull();
  });

  it("clears reconnecting banner when audio is lost", () => {
    const { result, rerender } = renderHook(
      ({ status }) => useAudioDeviceNotice(status, "outbound"),
      { initialProps: { status: baseStatus } },
    );

    rerender({
      status: { ...baseStatus, outboundAudio: "reconnecting" },
    });
    act(() => {
      vi.advanceTimersByTime(AUDIO_RECONNECT_DEBOUNCE_MS);
    });
    expect(result.current.mode).toBe("reconnecting");

    rerender({
      status: { ...baseStatus, outbound: "error", outboundAudio: "lost" },
    });
    expect(result.current.mode).toBeNull();
  });

  it("shows reconnecting for inbound meeting column", () => {
    const inboundDirectStatus: AppStatus = {
      ...baseStatus,
      outbound: "off",
      inbound: "direct",
    };

    const { result, rerender } = renderHook(
      ({ status }) => useAudioDeviceNotice(status, "inbound"),
      { initialProps: { status: inboundDirectStatus } },
    );

    rerender({
      status: {
        ...inboundDirectStatus,
        inboundAudio: "reconnecting",
        inboundAudioReconnectAttempt: 2,
      },
    });
    act(() => {
      vi.advanceTimersByTime(AUDIO_RECONNECT_DEBOUNCE_MS);
    });
    expect(result.current.mode).toBe("reconnecting");
  });
});
