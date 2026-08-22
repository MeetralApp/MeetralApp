import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import { usePipelineSessionElapsed } from "./usePipelineSessionElapsed";

const baseStatus: AppStatus = {
  running: true,
  outbound: "active",
  inbound: "off",
  error: null,
  outboundBridge: "ready",
  outboundActiveSince: 0,
};

describe("usePipelineSessionElapsed", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(65_000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows Starting… while pipeline is starting", () => {
    const { result } = renderHook(() =>
      usePipelineSessionElapsed(
        { ...baseStatus, outbound: "starting" },
        "outbound",
      ),
    );

    expect(result.current).toEqual({
      display: "Starting…",
      variant: "starting",
      isLongSession: false,
      title: null,
    });
  });

  it("shows Stopping… while pipeline is stopping", () => {
    const { result } = renderHook(() =>
      usePipelineSessionElapsed(
        { ...baseStatus, outbound: "stopping" },
        "outbound",
      ),
    );

    expect(result.current).toEqual({
      display: "Stopping…",
      variant: "stopping",
      isLongSession: false,
      title: null,
    });
  });

  it("shows Reconnecting… immediately when bridge is reconnecting", () => {
    const { result } = renderHook(() =>
      usePipelineSessionElapsed(
        { ...baseStatus, outboundBridge: "reconnecting" },
        "outbound",
      ),
    );

    expect(result.current).toEqual({
      display: "Reconnecting…",
      variant: "reconnecting",
      isLongSession: false,
      title: "Translation connection reconnecting",
    });
  });

  it("shows elapsed time when active with activeSince", () => {
    const { result } = renderHook(() =>
      usePipelineSessionElapsed(
        { ...baseStatus, outboundActiveSince: 60_000 },
        "outbound",
      ),
    );

    expect(result.current.variant).toBe("elapsed");
    expect(result.current.display).toBe("0:05");
    expect(result.current.isLongSession).toBe(false);
  });

  it("returns null when pipeline is idle", () => {
    const { result } = renderHook(() =>
      usePipelineSessionElapsed(
        { ...baseStatus, outbound: "off", outboundActiveSince: null },
        "outbound",
      ),
    );

    expect(result.current).toEqual({
      display: null,
      variant: null,
      isLongSession: false,
      title: null,
    });
  });
});
