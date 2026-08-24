import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  listElevenLabsModels,
  listElevenLabsVoices,
  listFishAudioModels,
  listFishAudioVoices,
  listSonioxVoices,
  previewElevenLabsVoice,
  previewFishAudioVoice,
  previewSonioxVoice,
  testElevenLabsApiKey,
  testFishAudioApiKey,
  testXaiApiKey,
  listXaiVoices,
  previewXaiVoice,
  validateElevenLabsVoice,
  validateFishAudioVoice,
  validateXaiVoice,
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

  it("lists, validates, and previews Fish Audio voices", async () => {
    await testFishAudioApiKey("fa-key");
    expect(invoke).toHaveBeenCalledWith("test_fishaudio_api_key", {
      request: { apiKey: "fa-key" },
    });
    await listFishAudioVoices("k");
    expect(invoke).toHaveBeenCalledWith("list_fishaudio_voices", {
      request: { apiKey: "k" },
    });
    await listFishAudioModels();
    expect(invoke).toHaveBeenCalledWith("list_fishaudio_models");
    await validateFishAudioVoice("v1", "k");
    expect(invoke).toHaveBeenCalledWith("validate_fishaudio_voice", {
      request: { apiKey: "k", voiceId: "v1" },
    });
    await previewFishAudioVoice("v1", "k");
    expect(invoke).toHaveBeenCalledWith("preview_fishaudio_voice", {
      request: { apiKey: "k", voiceId: "v1" },
    });
  });

  it("lists, validates, and previews xAI voices", async () => {
    await testXaiApiKey("xai-key");
    expect(invoke).toHaveBeenCalledWith("test_xai_api_key", {
      request: { apiKey: "xai-key" },
    });
    await listXaiVoices("k");
    expect(invoke).toHaveBeenCalledWith("list_xai_voices", {
      request: { apiKey: "k" },
    });
    await validateXaiVoice("eve", "k");
    expect(invoke).toHaveBeenCalledWith("validate_xai_voice", {
      request: { apiKey: "k", voiceId: "eve" },
    });
    await previewXaiVoice("eve", "k");
    expect(invoke).toHaveBeenCalledWith("preview_xai_voice", {
      request: { apiKey: "k", voiceId: "eve" },
    });
  });
});
