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
  });
});
