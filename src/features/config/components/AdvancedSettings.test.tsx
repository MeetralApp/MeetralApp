import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { ThemeProvider } from "@/shared/context/ThemeProvider";
import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import AdvancedSettings from "./AdvancedSettings";

function renderSettings(
  props: Partial<ComponentProps<typeof AdvancedSettings>> = {},
) {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const onToast = vi.fn();
  render(
    <ThemeProvider>
      <TooltipProvider>
        <AdvancedSettings
          config={baseConfig}
          onSave={onSave}
          onToast={onToast}
          {...props}
        />
      </TooltipProvider>
    </ThemeProvider>,
  );
  return { onSave, onToast };
}

describe("AdvancedSettings auto-end meeting", () => {
  it("selects Off by default", () => {
    renderSettings();
    expect(
      screen.getByRole("button", { name: "Off" }).getAttribute("aria-pressed"),
    ).toBe("true");
  });

  it("presses the matching minute preset when auto-end is on", () => {
    renderSettings({
      config: { ...baseConfig, autoEndMeeting: true, autoEndMeetingAfterMin: 5 },
    });
    expect(
      screen.getByRole("button", { name: "5" }).getAttribute("aria-pressed"),
    ).toBe("true");
    expect(
      screen.getByRole("button", { name: "Off" }).getAttribute("aria-pressed"),
    ).toBe("false");
  });

  it("persists a minute preset immediately", () => {
    const { onSave } = renderSettings();
    fireEvent.click(screen.getByRole("button", { name: "10" }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        autoEndMeeting: true,
        autoEndMeetingAfterMin: 10,
      }),
    );
  });
});
