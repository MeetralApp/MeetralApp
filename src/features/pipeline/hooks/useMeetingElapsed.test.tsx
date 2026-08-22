import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";

import { ClockProvider } from "@/shared/context/ClockProvider";
import { useMeetingElapsed } from "./useMeetingElapsed";

function wrapper({ children }: { children: ReactNode }) {
  return <ClockProvider>{children}</ClockProvider>;
}

describe("useMeetingElapsed", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-16T12:00:00.000Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns null without startedAtMs", () => {
    const { result } = renderHook(() => useMeetingElapsed(null), { wrapper });
    expect(result.current.display).toBeNull();
  });

  it("formats elapsed from startedAtMs", () => {
    const started = Date.now() - 65_000;
    const { result } = renderHook(() => useMeetingElapsed(started), {
      wrapper,
    });
    expect(result.current.display).toBe("1:05");
  });

  it("freezes when endedAtMs is set", () => {
    const started = Date.parse("2026-07-16T11:00:00.000Z");
    const ended = Date.parse("2026-07-16T11:10:00.000Z");
    const { result } = renderHook(() => useMeetingElapsed(started, ended), {
      wrapper,
    });
    expect(result.current.display).toBe("10:00");
  });
});
