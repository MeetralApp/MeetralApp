import { describe, expect, it } from "vitest";
import {
  engineVoiceHint,
  FALLBACK_FISH_MODELS,
  FALLBACK_SONIOX_VOICES,
  inboundVoiceNote,
  OUTPUT_OPTIONS,
  SYNTHESIS_MODE_OPTIONS,
} from "./voiceSettings";
import { normalizeCustomVoiceVendor } from "@/shared/lib/types/pipeline";

describe("voiceSettings", () => {
  it("exposes outbound engine options", () => {
    expect(OUTPUT_OPTIONS.map((o) => o.value)).toEqual([
      "providerNative",
      "custom",
    ]);
  });

  it("engineVoiceHint differs for soniox", () => {
    expect(engineVoiceHint("soniox")).toContain("Soniox TTS");
    expect(engineVoiceHint("gemini")).toContain("Gemini or OpenAI");
  });

  it("inboundVoiceNote differs for soniox", () => {
    expect(inboundVoiceNote("soniox")).toContain("Soniox TTS");
    expect(inboundVoiceNote("openAi")).toMatch(/built-in/i);
  });

  it("lists fallback soniox voices and synthesis modes", () => {
    expect(FALLBACK_SONIOX_VOICES.length).toBeGreaterThanOrEqual(3);
    expect(SYNTHESIS_MODE_OPTIONS.map((o) => o.value)).toEqual([
      "streaming",
      "sentence",
    ]);
  });

  it("seeds Fish Audio models including s2.1-pro", () => {
    expect(FALLBACK_FISH_MODELS.map((m) => m.modelId)).toContain("s2.1-pro");
  });
});

describe("normalizeCustomVoiceVendor", () => {
  it("keeps xAI and Fish Audio and falls back to ElevenLabs", () => {
    expect(normalizeCustomVoiceVendor("xai")).toBe("xai");
    expect(normalizeCustomVoiceVendor("fishAudio")).toBe("fishAudio");
    expect(normalizeCustomVoiceVendor("elevenLabs")).toBe("elevenLabs");
    expect(normalizeCustomVoiceVendor(undefined)).toBe("elevenLabs");
    expect(normalizeCustomVoiceVendor("")).toBe("elevenLabs");
  });
});
