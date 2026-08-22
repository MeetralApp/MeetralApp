import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

const unlisten = vi.fn();
const listen = vi.fn(async (_event: string, handler: () => void) => {
  listenHandler = handler;
  return unlisten;
});
let listenHandler: (() => void) | null = null;

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listen(...(args as [string, () => void])),
}));

import { useMeetingChanged } from "./useMeetingChanged";

describe("useMeetingChanged", () => {
  beforeEach(() => {
    listen.mockClear();
    unlisten.mockClear();
    listenHandler = null;
  });

  afterEach(() => {
  // Ensure singleton tears down between tests.
  });

  it("starts a single listener for multiple subscribers", async () => {
    const a = vi.fn();
    const b = vi.fn();
    const first = renderHook(() => useMeetingChanged(a));
    const second = renderHook(() => useMeetingChanged(b));

    await waitFor(() => expect(listen).toHaveBeenCalledTimes(1));
    expect(listen).toHaveBeenCalledWith("meeting-changed", expect.any(Function));

    act(() => {
      listenHandler?.();
    });
    expect(a).toHaveBeenCalled();
    expect(b).toHaveBeenCalled();

    first.unmount();
    expect(unlisten).not.toHaveBeenCalled();
    second.unmount();
    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });
});
