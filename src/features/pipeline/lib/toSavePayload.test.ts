import { describe, expect, it } from "vitest";
import { toSavePayload } from "./toSavePayload";
import { baseConfig } from "@/test/fixtures/config";

describe("toSavePayload", () => {
  it("maps config fields and omits empty api keys by default", () => {
    const payload = toSavePayload(baseConfig);
    expect(payload.aiProvider).toBe("gemini");
    expect(payload.myLanguage).toBe("en");
    expect(payload.meetingLanguage).toBe("vi");
    expect(payload.geminiApiKey).toBe("");
    expect(payload.openaiApiKey).toBe("");
    expect(payload.sonioxApiKey).toBe("");
    expect(payload.elevenlabsApiKey).toBe("");
    expect(payload.inboundVoiceOutput).toBe("providerNative");
    expect(payload.outboundVoiceOutput).toBe("providerNative");
  });

  it("applies option overrides for keys and voice", () => {
    const payload = toSavePayload(baseConfig, {
      geminiApiKey: "g-key",
      clearOpenaiApiKey: true,
      outboundVoiceOutput: "elevenLabsClone",
      inboundVoiceOutput: "elevenLabsClone",
      elevenlabsInboundVoiceId: "voice-in",
      elevenlabsInboundTtsModel: "model-in",
      elevenlabsInboundStability: 0.4,
      elevenlabsInboundSimilarityBoost: 0.8,
      elevenlabsInboundTtsSynthesisMode: "sentence",
      elevenlabsVoiceId: "voice-1",
      sonioxActiveContextProfileId: null,
    });
    expect(payload.geminiApiKey).toBe("g-key");
    expect(payload.clearOpenaiApiKey).toBe(true);
    expect(payload.outboundVoiceOutput).toBe("elevenLabsClone");
    expect(payload.inboundVoiceOutput).toBe("elevenLabsClone");
    expect(payload.elevenlabsInboundVoiceId).toBe("voice-in");
    expect(payload.elevenlabsInboundTtsModel).toBe("model-in");
    expect(payload.elevenlabsInboundStability).toBe(0.4);
    expect(payload.elevenlabsInboundSimilarityBoost).toBe(0.8);
    expect(payload.elevenlabsInboundTtsSynthesisMode).toBe("sentence");
    expect(payload.elevenlabsVoiceId).toBe("voice-1");
    expect(payload.sonioxActiveContextProfileId).toBe("");
  });

  it("preserves soniox context overrides", () => {
    const payload = toSavePayload(baseConfig, {
      sonioxAlwaysOn: {
        general: [{ key: "k", value: "v" }],
        text: "notes",
        terms: ["API"],
        translationTerms: [],
      },
    });
    expect(payload.sonioxAlwaysOn).toEqual({
      general: [{ key: "k", value: "v" }],
      text: "notes",
      terms: ["API"],
      translationTerms: [],
    });
  });

  it("passes through meetingContext only when explicitly provided", () => {
    // Absent when neither config nor options carries it → backend keeps existing.
    expect(toSavePayload(baseConfig).meetingContext).toBeUndefined();
    // Config holds a value, but no explicit save-options → omit (backend keeps).
    const withContext = {
      ...baseConfig,
      meetingContext: {
        general: [{ key: "domain", value: "Healthcare" }],
        text: "",
        terms: ["MRI"],
        translationTerms: [{ source: "stroke", target: "ictus" }],
      },
    };
    expect(toSavePayload(withContext).meetingContext).toBeUndefined();
    // Explicit option override wins — user just edited the field.
    const cleared = { general: [], text: "", terms: [], translationTerms: [] };
    expect(
      toSavePayload(withContext, { meetingContext: cleared }).meetingContext,
    ).toEqual(cleared);
  });

  it("persists live and TTS catalogs and clearSonioxApiKey", () => {
    const models = [
      {
        id: "stt-rt-v5",
        languages: [{ code: "en", name: "English", countryCode: "US" }],
      },
    ];
    const ttsModels = [
      {
        id: "tts-rt-v1",
        languages: [{ code: "vi", name: "Vietnamese", countryCode: "VN" }],
      },
    ];
    const payload = toSavePayload(baseConfig, {
      clearSonioxApiKey: true,
      sonioxLiveModels: models,
      geminiLiveModels: models,
      openAiLiveModels: models,
      sonioxTtsModels: ttsModels,
      sonioxTtsInboundSpeed: 1.15,
      sonioxTtsOutboundSpeed: 0.9,
    });
    expect(payload.clearSonioxApiKey).toBe(true);
    expect(payload.sonioxLiveModels).toEqual(models);
    expect(payload.geminiLiveModels).toEqual(models);
    expect(payload.openAiLiveModels).toEqual(models);
    expect(payload.sonioxTtsModels).toEqual(ttsModels);
    expect(payload.sonioxTtsInboundSpeed).toBe(1.15);
    expect(payload.sonioxTtsOutboundSpeed).toBe(0.9);
  });
});
