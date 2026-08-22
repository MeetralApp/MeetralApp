import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  listElevenLabsModels,
  listElevenLabsVoices,
  listSonioxVoices,
  previewElevenLabsVoice,
  previewSonioxVoice,
  testElevenLabsApiKey,
  validateElevenLabsVoice,
} from "./voiceApi";

describe("voiceApi", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it("testElevenLabsApiKey", async () => {
    await testElevenLabsApiKey("el-key");
    expect(invoke).toHaveBeenCalledWith("test_elevenlabs_api_key", {
      request: { apiKey: "el-key" },
    });
  });

  it("list and validate elevenlabs voices/models", async () => {
    await listElevenLabsVoices("k");
    expect(invoke).toHaveBeenCalledWith("list_elevenlabs_voices", {
      request: { apiKey: "k" },
    });
    await listElevenLabsModels("k");
    expect(invoke).toHaveBeenCalledWith("list_elevenlabs_models", {
      request: { apiKey: "k" },
    });
    await validateElevenLabsVoice("v1", "k");
    expect(invoke).toHaveBeenCalledWith("validate_elevenlabs_voice", {
      request: { apiKey: "k", voiceId: "v1" },
    });
    await previewElevenLabsVoice("v1", "k");
    expect(invoke).toHaveBeenCalledWith("preview_elevenlabs_voice", {
      request: { apiKey: "k", voiceId: "v1" },
    });
  });

  it("list and preview soniox voices", async () => {
    await listSonioxVoices("s");
    expect(invoke).toHaveBeenCalledWith("list_soniox_voices", {
      request: { apiKey: "s", preferredModel: null },
    });
    await listSonioxVoices("s", "tts-rt-v1");
    expect(invoke).toHaveBeenCalledWith("list_soniox_voices", {
      request: { apiKey: "s", preferredModel: "tts-rt-v1" },
    });
    await previewSonioxVoice("Adrian", "s");
    expect(invoke).toHaveBeenCalledWith("preview_soniox_voice", {
      request: { apiKey: "s", voice: "Adrian" },
    });
  });
});
