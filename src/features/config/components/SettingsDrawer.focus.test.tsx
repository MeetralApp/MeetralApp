import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { baseConfig, baseStatus } from "@/test/fixtures/config";
import SettingsDrawer from "./SettingsDrawer";

vi.mock("@/shared/context/useToast", () => ({
  useToast: () => ({ showToast: vi.fn() }),
}));

vi.mock("./settingsDrawerTabs", () => ({
  TranslateTabPanel: () => <div>Translate panel</div>,
  IntelligenceTabPanel: () => <div>Intelligence panel</div>,
  VoiceTabPanel: () => <div>Voice panel</div>,
  AudioTabPanel: () => <div>Audio panel</div>,
  AppTabPanel: () => <div>App panel</div>,
}));

const readyAudio = {
  outboundReady: true,
  inboundReady: true,
  roles: [],
};

function renderDrawer(
  props: Partial<ComponentProps<typeof SettingsDrawer>> = {},
) {
  return render(
    <SettingsDrawer
      open
      onClose={vi.fn()}
      config={baseConfig}
      status={baseStatus}
      devices={null}
      audioSetup={readyAudio}
      onSave={vi.fn().mockResolvedValue(undefined)}
      onSaveLanguages={vi.fn().mockResolvedValue(undefined)}
      onTestApiKey={vi.fn().mockResolvedValue(undefined)}
      onRefreshDevices={vi.fn().mockResolvedValue(undefined)}
      {...props}
    />,
  );
}

describe("SettingsDrawer focus routing", () => {
  it("opens the App tab when focus is overlay", () => {
    renderDrawer({ settingsFocus: "overlay" });

    expect(screen.getByText("App panel")).toBeTruthy();
    expect(screen.queryByText("Translate panel")).toBeNull();
  });

  it("maps legacy api focus onto Translate tab", () => {
    renderDrawer({ settingsFocus: "api" });

    expect(screen.getByText("Translate panel")).toBeTruthy();
  });

  it("closes immediately when there are no unsaved changes", () => {
    const onClose = vi.fn();
    renderDrawer({ onClose });

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
