import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

const showToast = vi.fn();
let statusHandler:
  | ((event: { payload: { kind: string; message?: string } }) => void)
  | null = null;

vi.mock("@/shared/context/useToast", () => ({
  useToast: () => ({ showToast }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event: string, handler: typeof statusHandler) => {
    statusHandler = handler;
    return () => {
      statusHandler = null;
    };
  }),
}));

import { useVoiceTtsStatus } from "./useVoiceTtsStatus";

describe("useVoiceTtsStatus", () => {
  beforeEach(() => {
    showToast.mockReset();
    statusHandler = null;
  });

  it("resets when disabled", async () => {
    const { result, rerender } = renderHook(
      ({ enabled }) => useVoiceTtsStatus(enabled),
      { initialProps: { enabled: true } },
    );

    await waitFor(() => expect(statusHandler).not.toBeNull());
    act(() => {
      statusHandler?.({ payload: { kind: "ready" } });
    });
    expect(result.current.ready).toBe(true);

    rerender({ enabled: false });
    expect(result.current.ready).toBe(false);
    expect(result.current.degradedMessage).toBeNull();
  });

  it("handles ready and degraded events", async () => {
    const { result } = renderHook(() => useVoiceTtsStatus(true));
    await waitFor(() => expect(statusHandler).not.toBeNull());

    act(() => {
      statusHandler?.({ payload: { kind: "ready" } });
    });
    expect(result.current.ready).toBe(true);
    expect(result.current.degradedMessage).toBeNull();

    act(() => {
      statusHandler?.({
        payload: { kind: "degraded", message: "TTS offline" },
      });
    });
    expect(result.current.ready).toBe(false);
    expect(result.current.degradedMessage).toBe("TTS offline");
    expect(showToast).toHaveBeenCalledWith("error", "TTS offline");
  });
});
