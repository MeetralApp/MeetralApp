import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import {
  DEFAULT_OVERLAY_SETTINGS,
  type OverlaySettings as OverlayConfig,
} from "@/shared/lib/types/pipeline";
import OverlaySettings from "./OverlaySettings";

vi.mock("@/features/meeting/library/hooks/useActiveMeeting", () => ({
  useActiveMeeting: () => ({ activeMeeting: null }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ label: "main" }),
}));

function renderSettings(overlay?: Partial<OverlayConfig>) {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const config = {
    ...baseConfig,
    overlay: { ...DEFAULT_OVERLAY_SETTINGS, ...overlay },
  };
  render(
    <TooltipProvider>
      <OverlaySettings config={config} onSave={onSave} />
    </TooltipProvider>,
  );
  return onSave;
}

describe("OverlaySettings", () => {
  it("defaults to Off (factory default disables the overlay)", () => {
    renderSettings();
    expect(
      screen.getByRole("button", { name: "Off" }).getAttribute("aria-pressed"),
    ).toBe("true");
    expect(
      screen.getByRole("button", { name: "On" }).getAttribute("aria-pressed"),
    ).toBe("false");
  });

  it("reflects On when enabled", () => {
    renderSettings({ enabled: true });
    expect(
      screen.getByRole("button", { name: "On" }).getAttribute("aria-pressed"),
    ).toBe("true");
  });

  it("saves enabled when switching On", () => {
    const onSave = renderSettings({ enabled: false });
    fireEvent.click(screen.getByRole("button", { name: "On" }));
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onSave.mock.calls[0][0].overlay.enabled).toBe(true);
  });

  it("clears enabled when switching Off", () => {
    const onSave = renderSettings({ enabled: true });
    fireEvent.click(screen.getByRole("button", { name: "Off" }));
    expect(onSave.mock.calls[0][0].overlay.enabled).toBe(false);
  });
});
