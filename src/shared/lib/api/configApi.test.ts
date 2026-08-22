import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { saveConfig, testApiKey } from "./configApi";
import type { SaveConfigPayload } from "@/shared/lib/types/pipeline";

describe("configApi", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("saveConfig invokes save_config with payload", async () => {
    const config = { aiProvider: "gemini" } as SaveConfigPayload;
    vi.mocked(invoke).mockResolvedValue({ ok: true });
    await saveConfig(config);
    expect(invoke).toHaveBeenCalledWith("save_config", { config });
  });

  it("testApiKey invokes test_api_key", async () => {
    await testApiKey("secret", "openAi");
    expect(invoke).toHaveBeenCalledWith("test_api_key", {
      request: { apiKey: "secret", provider: "openAi", purpose: "live" },
    });
  });

  it("testApiKey passes summary purpose", async () => {
    await testApiKey("secret", "gemini", "summary");
    expect(invoke).toHaveBeenCalledWith("test_api_key", {
      request: { apiKey: "secret", provider: "gemini", purpose: "summary" },
    });
  });
});
