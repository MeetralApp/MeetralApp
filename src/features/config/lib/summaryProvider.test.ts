import { describe, expect, it } from "vitest";

import { baseConfig } from "@/test/fixtures/config";
import type { CustomLlmProfileView } from "@/shared/lib/types/pipeline";
import {
  resolveLlmSelection,
  resolveSummaryProvider,
  selectionLabel,
  selectionUsable,
  summaryKeyConfigured,
} from "./summaryProvider";

const ollama: CustomLlmProfileView = {
  id: "p1",
  label: "Ollama (local)",
  baseUrl: "http://localhost:11434",
  chatModel: "qwen3:8b",
  apiKeyConfigured: false,
};

describe("resolveSummaryProvider", () => {
  it("maps openAi and falls back to gemini", () => {
    expect(
      resolveSummaryProvider({ ...baseConfig, summaryProvider: "openAi" }),
    ).toBe("openAi");
    expect(
      resolveSummaryProvider({ ...baseConfig, summaryProvider: "gemini" }),
    ).toBe("gemini");
    expect(resolveSummaryProvider({ ...baseConfig, summaryProvider: undefined })).toBe(
      "gemini",
    );
  });
});

describe("resolveLlmSelection", () => {
  it("returns builtin when no custom profile is selected", () => {
    const selection = resolveLlmSelection({
      ...baseConfig,
      summaryProvider: "openAi",
      customLlmProfiles: [ollama],
      summaryCustomProfileId: null,
    });
    expect(selection).toEqual({ kind: "builtin", provider: "openAi" });
  });

  it("returns the custom profile when selected", () => {
    const selection = resolveLlmSelection({
      ...baseConfig,
      customLlmProfiles: [ollama],
      summaryCustomProfileId: "p1",
    });
    expect(selection).toEqual({ kind: "custom", profile: ollama });
  });

  it("falls back to builtin when the selected profile is missing", () => {
    const selection = resolveLlmSelection({
      ...baseConfig,
      summaryProvider: "gemini",
      customLlmProfiles: [],
      summaryCustomProfileId: "deleted-id",
    });
    expect(selection).toEqual({ kind: "builtin", provider: "gemini" });
  });
});

describe("selectionUsable", () => {
  it("builtin requires the provider key", () => {
    expect(
      selectionUsable({ ...baseConfig, summaryProvider: "gemini", geminiApiKeyConfigured: false }),
    ).toBe(false);
    expect(
      selectionUsable({ ...baseConfig, summaryProvider: "gemini", geminiApiKeyConfigured: true }),
    ).toBe(true);
    expect(
      selectionUsable({ ...baseConfig, summaryProvider: "openAi", openaiApiKeyConfigured: true }),
    ).toBe(true);
  });

  it("custom selection is usable once saved (save-time test passed)", () => {
    expect(
      selectionUsable({
        ...baseConfig,
        geminiApiKeyConfigured: false,
        customLlmProfiles: [ollama],
        summaryCustomProfileId: "p1",
      }),
    ).toBe(true);
  });

  it("missing selected profile degrades to the builtin key check", () => {
    expect(
      selectionUsable({
        ...baseConfig,
        summaryProvider: "gemini",
        geminiApiKeyConfigured: false,
        customLlmProfiles: [],
        summaryCustomProfileId: "gone",
      }),
    ).toBe(false);
  });
});

describe("selectionLabel", () => {
  it("labels builtins and customs", () => {
    expect(selectionLabel({ kind: "builtin", provider: "gemini" })).toBe("Gemini");
    expect(selectionLabel({ kind: "builtin", provider: "openAi" })).toBe("OpenAI");
    expect(selectionLabel({ kind: "custom", profile: ollama })).toBe("Ollama (local)");
  });
});

describe("summaryKeyConfigured (legacy helper)", () => {
  it("checks the resolved builtin provider key", () => {
    expect(
      summaryKeyConfigured({ ...baseConfig, summaryProvider: "openAi", openaiApiKeyConfigured: true }),
    ).toBe(true);
    expect(
      summaryKeyConfigured({ ...baseConfig, summaryProvider: "openAi", openaiApiKeyConfigured: false }),
    ).toBe(false);
  });
});
