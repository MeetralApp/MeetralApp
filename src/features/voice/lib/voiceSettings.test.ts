import { describe, expect, it } from "vitest";
import {
  engineVoiceHint,
  FALLBACK_SONIOX_VOICES,
  inboundVoiceNote,
  OUTPUT_OPTIONS,
  SYNTHESIS_MODE_OPTIONS,
} from "./voiceSettings";

describe("voiceSettings", () => {
  it("exposes outbound engine options", () => {
    expect(OUTPUT_OPTIONS.map((o) => o.value)).toEqual([
      "providerNative",
      "elevenLabsClone",
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
});
