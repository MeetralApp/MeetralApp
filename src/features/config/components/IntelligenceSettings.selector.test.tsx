import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import type { CustomLlmProfileView } from "@/shared/lib/types/pipeline";
import IntelligenceSettings from "./IntelligenceSettings";

vi.mock("@/features/ai/hooks/useAiCatalog", () => ({
  useAiCatalog: () => ({
    summaryModels: [{ id: "m1", label: "Model 1", description: "" }],
    defaults: { summaryModel: "m1" },
  }),
  loadAiCatalog: vi.fn(async () => ({
    summaryModels: [{ id: "m1", label: "Model 1", description: "" }],
    defaults: { summaryModel: "m1" },
  })),
}));

vi.mock("@/shared/components/SettingInfoHint", () => ({
  default: () => null,
}));

vi.mock("@/shared/components/SecretApiKeyField", () => ({
  default: () => null,
}));

vi.mock("@/features/meeting/library/lib/meetingApi", () => ({
  listSummaryLanguages: vi.fn(async () => []),
}));

vi.mock("@/shared/lib/api/configApi", () => ({
  upsertCustomLlmProfile: vi.fn(),
  deleteCustomLlmProfile: vi.fn(),
  testCustomLlmProfile: vi.fn(),
}));

const ollama: CustomLlmProfileView = {
  id: "p1",
  label: "Ollama (local)",
  baseUrl: "http://localhost:11434",
  chatModel: "qwen3:8b",
  apiKeyConfigured: false,
};

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

function renderSettings(configOverrides: Record<string, unknown> = {}) {
  const onSave = vi.fn().mockResolvedValue(undefined);
  render(
    wrap(
      <IntelligenceSettings
        config={{
          ...baseConfig,
          summaryProvider: "gemini",
          geminiApiKeyConfigured: true,
          ...configOverrides,
        }}
        onSave={onSave}
        onTestApiKey={vi.fn()}
        onToast={vi.fn()}
      />,
    ),
  );
  return { onSave };
}

describe("IntelligenceSettings provider selector", () => {
  it("lists custom profiles in the provider select", async () => {
    renderSettings({ customLlmProfiles: [ollama] });
    const trigger = document.getElementById("settings-summary-provider");
    fireEvent.keyDown(trigger as HTMLElement, { key: "ArrowDown" });
    expect(
      await screen.findByRole("option", { name: "Ollama (local) (custom)" }),
    ).toBeTruthy();
    expect(screen.getByRole("option", { name: "Gemini" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "OpenAI" })).toBeTruthy();
  });

  it("selecting a custom profile persists summaryCustomProfileId", async () => {
    const { onSave } = renderSettings({ customLlmProfiles: [ollama] });

    const trigger = document.getElementById("settings-summary-provider");
    expect(trigger).toBeTruthy();
    fireEvent.keyDown(trigger as HTMLElement, { key: "ArrowDown" });
    const option = await screen.findByRole("option", {
      name: "Ollama (local) (custom)",
    });
    fireEvent.click(option);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });
    expect(onSave.mock.calls[0][0]).toMatchObject({
      summaryCustomProfileId: "p1",
    });
  });

  it("custom selection shows readonly details and hides the model dropdown", () => {
    renderSettings({
      customLlmProfiles: [ollama],
      summaryCustomProfileId: "p1",
    });

    // baseUrl shows in both the selection card and the profile list below.
    expect(
      screen.getAllByText("http://localhost:11434").length,
    ).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Model: qwen3:8b")).toBeTruthy();
    expect(
      document.getElementById("settings-summary-model"),
    ).toBeNull();
  });

  it("switching back to a built-in clears the custom selection", async () => {
    const { onSave } = renderSettings({
      customLlmProfiles: [ollama],
      summaryCustomProfileId: "p1",
    });

    const trigger = document.getElementById("settings-summary-provider");
    fireEvent.keyDown(trigger as HTMLElement, { key: "ArrowDown" });
    const option = await screen.findByRole("option", { name: "Gemini" });
    fireEvent.click(option);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });
    expect(onSave.mock.calls[0][0]).toMatchObject({
      summaryProvider: "gemini",
      summaryCustomProfileId: "",
    });
  });
});
