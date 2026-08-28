import { describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";

vi.mock("@/shared/lib/api/voiceApi", () => ({
  listElevenLabsVoices: vi.fn(),
  listElevenLabsModels: vi.fn(),
  listSonioxVoices: vi.fn(),
  validateElevenLabsVoice: vi.fn(),
  previewElevenLabsVoice: vi.fn(),
  previewSonioxVoice: vi.fn(),
  testElevenLabsApiKey: vi.fn(),
  testFishAudioApiKey: vi.fn(),
  listFishAudioVoices: vi.fn(),
  listFishAudioModels: vi.fn(),
  validateFishAudioVoice: vi.fn(),
  previewFishAudioVoice: vi.fn(),
  testXaiApiKey: vi.fn(),
  listXaiVoices: vi.fn(),
  validateXaiVoice: vi.fn(),
  previewXaiVoice: vi.fn(),
}));

import * as voiceApi from "@/shared/lib/api/voiceApi";
import { useVoiceCatalog } from "./useVoiceCatalog";

describe("useVoiceCatalog", () => {
  it("exposes voiceApi list/preview/validate helpers", () => {
    const { result } = renderHook(() => useVoiceCatalog());
    expect(result.current.listElevenLabsVoices).toBe(voiceApi.listElevenLabsVoices);
    expect(result.current.listElevenLabsModels).toBe(voiceApi.listElevenLabsModels);
    expect(result.current.listSonioxVoices).toBe(voiceApi.listSonioxVoices);
    expect(result.current.validateElevenLabsVoice).toBe(
      voiceApi.validateElevenLabsVoice,
    );
    expect(result.current.previewElevenLabsVoice).toBe(
      voiceApi.previewElevenLabsVoice,
    );
    expect(result.current.previewSonioxVoice).toBe(voiceApi.previewSonioxVoice);
    expect(result.current.testElevenLabsApiKey).toBe(
      voiceApi.testElevenLabsApiKey,
    );
    expect(result.current.testFishAudioApiKey).toBe(voiceApi.testFishAudioApiKey);
    expect(result.current.listFishAudioVoices).toBe(voiceApi.listFishAudioVoices);
    expect(result.current.previewFishAudioVoice).toBe(
      voiceApi.previewFishAudioVoice,
    );
    expect(result.current.testXaiApiKey).toBe(voiceApi.testXaiApiKey);
    expect(result.current.listXaiVoices).toBe(voiceApi.listXaiVoices);
    expect(result.current.previewXaiVoice).toBe(voiceApi.previewXaiVoice);
  });
});
