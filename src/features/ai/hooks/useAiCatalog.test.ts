import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

const getAiCatalog = vi.fn();

vi.mock("../lib/aiApi", () => ({
  getAiCatalog: (...args: unknown[]) => getAiCatalog(...args),
  migrateConfigForProvider: vi.fn(),
}));

import { invalidateAiCatalog, loadAiCatalog, useAiCatalog } from "./useAiCatalog";

describe("useAiCatalog / loadAiCatalog", () => {
  beforeEach(() => {
    invalidateAiCatalog();
    getAiCatalog.mockReset();
  });

  it("returns fetched catalog and caches it", async () => {
    getAiCatalog.mockResolvedValue({
      provider: "gemini",
      languages: [{ code: "vi", name: "Vietnamese", countryCode: "VN" }],
      liveModels: [],
      summaryModels: [],
      defaults: { liveModel: "m", summaryModel: "s" },
      capabilities: {
        supportsVadConfig: true,
        supportsEchoTargetLanguage: true,
        liveUploadSampleRate: 16000,
      },
    });

    const catalog = await loadAiCatalog("gemini");
    expect(catalog.languages[0]?.code).toBe("vi");
    expect(getAiCatalog).toHaveBeenCalledTimes(1);

    await loadAiCatalog("gemini");
    expect(getAiCatalog).toHaveBeenCalledTimes(1);
  });

  it("falls back when getAiCatalog rejects", async () => {
    getAiCatalog.mockRejectedValue(new Error("offline"));
    const catalog = await loadAiCatalog("soniox");
    expect(catalog.defaults.liveModel).toBe("stt-rt-v5");
    expect(catalog.capabilities.usesSeparateTts).toBe(true);
    expect(catalog.capabilities.supportsNotesSttOnly).toBe(true);
  });

  it("OpenAI fallback exposes notes STT whisper separate from live translate", async () => {
    getAiCatalog.mockRejectedValue(new Error("offline"));
    invalidateAiCatalog("openAi");
    const catalog = await loadAiCatalog("openAi");
    expect(catalog.notesSttModel?.id).toBe("gpt-realtime-whisper");
    expect(catalog.liveModels.some((m) => m.id === "gpt-realtime-translate")).toBe(
      true,
    );
    expect(catalog.liveModels.some((m) => m.id === "gpt-realtime-whisper")).toBe(
      false,
    );
  });

  it("useAiCatalog exposes catalog after load", async () => {
    getAiCatalog.mockResolvedValue({
      provider: "openAi",
      languages: [],
      liveModels: [{ id: "gpt-realtime-translate", label: "RT", description: "" }],
      summaryModels: [],
      defaults: {
        liveModel: "gpt-realtime-translate",
        summaryModel: "gpt-4o",
      },
      capabilities: {
        supportsVadConfig: false,
        supportsEchoTargetLanguage: false,
        liveUploadSampleRate: 24000,
      },
    });

    const { result } = renderHook(() => useAiCatalog("openAi"));
    await waitFor(() =>
      expect(result.current.defaults.liveModel).toBe("gpt-realtime-translate"),
    );
  });
});
