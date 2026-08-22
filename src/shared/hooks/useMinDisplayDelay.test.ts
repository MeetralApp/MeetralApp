import { renderHook, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useMinDisplayDelay } from "./useMinDisplayDelay";

describe("useMinDisplayDelay", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("stays visible for the minimum duration after loading ends", () => {
    const { result, rerender } = renderHook(
      ({ active }) => useMinDisplayDelay(active, 450),
      { initialProps: { active: true } },
    );

    expect(result.current).toBe(true);

    rerender({ active: false });
    expect(result.current).toBe(true);

    act(() => {
      vi.advanceTimersByTime(449);
    });
    expect(result.current).toBe(true);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current).toBe(false);
  });

  it("hides immediately when active was never true long enough to matter", () => {
    const { result, rerender } = renderHook(
      ({ active }) => useMinDisplayDelay(active, 450),
      { initialProps: { active: false } },
    );

    expect(result.current).toBe(false);

    rerender({ active: true });
    expect(result.current).toBe(true);

    rerender({ active: false });
    act(() => {
      vi.advanceTimersByTime(450);
    });
    expect(result.current).toBe(false);
  });
});
