import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import { TranslateTabPanel } from "./settingsDrawerTabs";

vi.mock("@/features/ai/hooks/useLiveModelCatalog", () => ({
  useLiveModelCatalog: () => ({
    models: [],
    languages: [],
    loading: false,
    refresh: vi.fn(),
  }),
}));

vi.mock("@/features/ai/components/ProviderSettings", () => ({
  default: () => <div data-testid="provider-settings" />,
}));

vi.mock("@/features/ai/components/ApiKeySettings", () => ({
  default: () => <div data-testid="api-key-settings" />,
}));

vi.mock("@/features/ai/components/AiModelSettings", () => ({
  default: () => <div data-testid="model-settings" />,
}));

vi.mock("./LanguageSettings", () => ({
  default: () => <div data-testid="language-settings" />,
}));

vi.mock("@/shared/components/SettingInfoHint", () => ({
  default: () => null,
}));

const readyStatus = { tone: "ok" as const, label: "Ready" };
const setupStatus = { tone: "warn" as const, label: "Setup needed" };

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

function renderPanel(
  overrides: {
    apiKeyConfigured?: boolean;
    effectiveFocus?: "api" | "provider" | null;
    translationStatus?: typeof readyStatus;
  } = {},
) {
  const apiKeyConfigured = overrides.apiKeyConfigured ?? true;
  return render(
    wrap(
      <TranslateTabPanel
        config={{ ...baseConfig, apiKeyConfigured }}
        languagesLocked={false}
        effectiveFocus={overrides.effectiveFocus ?? null}
        translationStatus={
          overrides.translationStatus ??
          (apiKeyConfigured ? readyStatus : setupStatus)
        }
        sonioxContextStatus={undefined}
        onSave={vi.fn().mockResolvedValue(undefined)}
        onSaveLanguages={vi.fn().mockResolvedValue(undefined)}
        onTestApiKey={vi.fn().mockResolvedValue(undefined)}
        onToast={vi.fn()}
        onApiKeyDirty={vi.fn()}
        onSonioxContextDirty={vi.fn()}
        onSpeechDetectionDirty={vi.fn()}
      />,
    ),
  );
}

describe("TranslateTabPanel API key chrome", () => {
  it("shows the API key section when the key is missing", () => {
    renderPanel({ apiKeyConfigured: false });

    expect(screen.getByText("API key")).toBeTruthy();
    expect(screen.getByTestId("api-key-settings")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: /Manage API key/ }),
    ).toBeNull();
  });

  it("hides the API key section and shows the Engine chip when configured", () => {
    renderPanel({ apiKeyConfigured: true });

    expect(screen.queryByTestId("api-key-settings")).toBeNull();
    expect(
      screen.getByRole("button", { name: /Manage API key \(Ready\)/ }),
    ).toBeTruthy();
  });

  it("expands the in-place API key panel from the Engine chip", () => {
    renderPanel({ apiKeyConfigured: true });

    fireEvent.click(
      screen.getByRole("button", { name: /Manage API key \(Ready\)/ }),
    );

    expect(screen.getByTestId("api-key-settings")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: /Hide API key \(Ready\)/ }),
    ).toBeTruthy();
  });

  it("opens the Engine API key panel when focus is api and key is configured", () => {
    renderPanel({ apiKeyConfigured: true, effectiveFocus: "api" });

    expect(screen.getByTestId("api-key-settings")).toBeTruthy();
  });
});
