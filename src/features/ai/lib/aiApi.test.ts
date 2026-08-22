import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getAiCatalog,
  languagesForLiveModel,
  liveModelsFromConfig,
  migrateConfigForProvider,
  persistKeyForProvider,
  seedLiveModelCatalog,
  listSonioxSttModels,
} from "./aiApi";
import type { ConfigView } from "@/shared/lib/types/pipeline";
import { baseConfig } from "@/test/fixtures/config";

describe("aiApi", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("getAiCatalog invokes get_ai_catalog", async () => {
    vi.mocked(invoke).mockResolvedValue({ provider: "gemini", languages: [] });
    await getAiCatalog("gemini");
    expect(invoke).toHaveBeenCalledWith("get_ai_catalog", { provider: "gemini" });
  });

  it("migrateConfigForProvider invokes migrate_config_for_provider", async () => {
    vi.mocked(invoke).mockResolvedValue({ aiProvider: "openAi" });
    await migrateConfigForProvider("openAi");
    expect(invoke).toHaveBeenCalledWith("migrate_config_for_provider", {
      provider: "openAi",
    });
  });

  it("seedLiveModelCatalog and listSonioxSttModels invoke commands", async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    await seedLiveModelCatalog("gemini");
    expect(invoke).toHaveBeenCalledWith("seed_live_model_catalog", {
      provider: "gemini",
    });
    await listSonioxSttModels("key");
    expect(invoke).toHaveBeenCalledWith("list_soniox_stt_models", {
      request: { apiKey: "key" },
    });
  });

  it("liveModelsFromConfig and persistKeyForProvider are provider-scoped", () => {
    const config = {
      ...baseConfig,
      geminiLiveModels: [{ id: "g1", languages: [] }],
      openAiLiveModels: [{ id: "o1", languages: [] }],
      sonioxLiveModels: [{ id: "s1", languages: [] }],
    } as ConfigView;
    expect(liveModelsFromConfig(config, "gemini")[0].id).toBe("g1");
    expect(liveModelsFromConfig(config, "openAi")[0].id).toBe("o1");
    expect(liveModelsFromConfig(config, "soniox")[0].id).toBe("s1");
    expect(persistKeyForProvider("gemini")).toBe("geminiLiveModels");
    expect(persistKeyForProvider("openAi")).toBe("openAiLiveModels");
    expect(persistKeyForProvider("soniox")).toBe("sonioxLiveModels");
  });

  it("languagesForLiveModel returns selected model languages", () => {
    const langs = languagesForLiveModel(
      [
        { id: "a", languages: [{ code: "en", name: "English", countryCode: "US" }] },
        { id: "b", languages: [{ code: "vi", name: "Vietnamese", countryCode: "VN" }] },
      ],
      "b",
    );
    expect(langs).toEqual([{ code: "vi", name: "Vietnamese", countryCode: "VN" }]);
    expect(languagesForLiveModel([], "missing")).toEqual([]);
  });
});
