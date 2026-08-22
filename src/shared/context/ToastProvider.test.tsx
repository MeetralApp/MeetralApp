import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ToastProvider } from "./ToastProvider";
import { TOAST_DURATION } from "./toastTypes";
import { useToast } from "./useToast";

function Probe() {
  const { showToast } = useToast();
  return (
    <div>
      <button type="button" onClick={() => showToast("success", "Saved")}>
        success
      </button>
      <button type="button" onClick={() => showToast("info", "Hello")}>
        info
      </button>
      <button type="button" onClick={() => showToast("error", "Failed")}>
        error
      </button>
    </div>
  );
}

describe("ToastProvider", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("shows a toast via showToast", () => {
    render(
      <ToastProvider>
        <Probe />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "success" }).click();
    });

    expect(screen.getByRole("status").textContent).toContain("Saved");
    expect(screen.getByRole("status").getAttribute("data-type")).toBe("success");
  });

  it("replaces the visible toast when showToast is called again", () => {
    render(
      <ToastProvider>
        <Probe />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "success" }).click();
    });
    expect(screen.getByRole("status").textContent).toContain("Saved");

    act(() => {
      screen.getByRole("button", { name: "info" }).click();
    });
    expect(screen.getByRole("status").textContent).toContain("Hello");
    expect(screen.queryByText("Saved")).toBeNull();
  });

  it("auto-dismisses after the type duration", () => {
    render(
      <ToastProvider>
        <Probe />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "error" }).click();
    });
    expect(screen.getByRole("status").textContent).toContain("Failed");

    act(() => {
      vi.advanceTimersByTime(TOAST_DURATION.error);
    });

    expect(screen.queryByRole("status")).toBeNull();
  });
});
