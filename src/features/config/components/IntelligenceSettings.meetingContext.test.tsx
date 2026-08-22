import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
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

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

describe("IntelligenceSettings Meeting context save", () => {
  it("passes meetingContext via save options so it is not dropped", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onToast = vi.fn();
    const saved = {
      general: [{ key: "domain", value: "Software engineering" }],
      text: "Engineering team meetings",
      terms: ["API", "PR"],
      translationTerms: [{ source: "deploy", target: "deploy" }],
    };

    render(
      wrap(
        <IntelligenceSettings
          config={{
            ...baseConfig,
            summaryProvider: "gemini",
            geminiApiKeyConfigured: true,
            meetingContext: saved,
          }}
          onSave={onSave}
          onTestApiKey={vi.fn()}
          onToast={onToast}
        />,
      ),
    );

    // Section starts collapsed — open it so the editor mounts with saved values.
    fireEvent.click(screen.getByRole("button", { name: /Meeting context/i }));

    // Dirty the draft so Save appears, then save.
    const textArea = await screen.findByDisplayValue("Engineering team meetings");
    fireEvent.change(textArea, {
      target: { value: "Engineering team meetings — sprint + incident" },
    });

    const saveButton = await screen.findByRole("button", { name: "Save" });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });

    expect(onSave.mock.calls[0][0].meetingContext).toEqual({
      general: [{ key: "domain", value: "Software engineering" }],
      text: "Engineering team meetings — sprint + incident",
      terms: ["API", "PR"],
      translationTerms: [{ source: "deploy", target: "deploy" }],
    });
    expect(onToast).toHaveBeenCalledWith("success", "Meeting context saved");
  });
});
