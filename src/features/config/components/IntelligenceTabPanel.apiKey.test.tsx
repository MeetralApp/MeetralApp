import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import { IntelligenceTabPanel } from "./settingsDrawerTabs";

vi.mock("@/features/ai/hooks/useAiCatalog", () => ({
  useAiCatalog: () => ({
    summaryModels: [{ id: "m1", label: "Model 1", description: "" }],
    defaults: { summaryModel: "m1" },
  }),
  loadAiCatalog: vi.fn(),
}));

vi.mock("@/shared/components/SettingInfoHint", () => ({
  default: () => null,
}));

vi.mock("@/shared/components/SecretApiKeyField", () => ({
  default: () => <div data-testid="secret-api-key-field" />,
}));

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

describe("IntelligenceTabPanel API key chrome", () => {
  it("shows the API key field when the summary provider key is missing", () => {
    render(
      wrap(
        <IntelligenceTabPanel
          config={{
            ...baseConfig,
            summaryProvider: "gemini",
            geminiApiKeyConfigured: false,
          }}
          intelligenceStatus={{ tone: "warn", label: "Setup needed" }}
          effectiveFocus={null}
          onSave={vi.fn()}
          onTestApiKey={vi.fn()}
          onToast={vi.fn()}
        />,
      ),
    );

    expect(screen.getByTestId("secret-api-key-field")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: /Manage API key/ }),
    ).toBeNull();
  });

  it("hides the field and shows a chip when the key is configured", () => {
    render(
      wrap(
        <IntelligenceTabPanel
          config={{
            ...baseConfig,
            summaryProvider: "gemini",
            geminiApiKeyConfigured: true,
          }}
          intelligenceStatus={{ tone: "ok", label: "Ready" }}
          effectiveFocus={null}
          onSave={vi.fn()}
          onTestApiKey={vi.fn()}
          onToast={vi.fn()}
        />,
      ),
    );

    expect(screen.queryByTestId("secret-api-key-field")).toBeNull();
    expect(screen.queryByRole("button", { name: "Ready" })).toBeNull();
    fireEvent.click(
      screen.getByRole("button", { name: /Manage API key \(Ready\)/ }),
    );
    expect(screen.getByTestId("secret-api-key-field")).toBeTruthy();
  });

  it("shows Setup needed badge without chip when key is missing", () => {
    render(
      wrap(
        <IntelligenceTabPanel
          config={{
            ...baseConfig,
            summaryProvider: "gemini",
            geminiApiKeyConfigured: false,
          }}
          intelligenceStatus={{ tone: "warn", label: "Setup needed" }}
          effectiveFocus={null}
          onSave={vi.fn()}
          onTestApiKey={vi.fn()}
          onToast={vi.fn()}
        />,
      ),
    );

    expect(screen.getByRole("button", { name: "Setup needed" })).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: /Manage API key/ }),
    ).toBeNull();
  });
});
