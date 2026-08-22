import { describe, expect, it } from "vitest";
import {
  formatDirectionChipLabel,
  formatOutboundToolbarShort,
  formatOutputModeShort,
  getOutboundToolbarModeOptions,
  getPipelineModeOptions,
  inboundToolbarModeFromConfig,
  inboundToolbarPatch,
  isInboundCloneReady,
  isOutboundCloneReady,
  outboundToolbarModeFromConfig,
} from "./pipelineLabels";
import type { AppStatus, ConfigView } from "@/shared/lib/types/pipeline";

const baseConfig: ConfigView = {
  aiProvider: "gemini",
  apiKeyConfigured: true,
  myLanguage: "vi",
  meetingLanguage: "en",
  userMic: { id: "mic", name: "Mic" },
  teamsMicFeed: { id: "teams", name: "Teams" },
  meetingCapture: { id: "cap", name: "Cap" },
  localPlayback: { id: "hp", name: "HP" },
  outboundMode: "originalAudio",
  inboundMode: "translated",
  liveModel: "gemini-3.5-live-translate-preview",
  summaryModel: "gemini-2.5-flash",
  echoTargetLanguage: true,
  vadSilenceDurationMs: 800,
  vadStartSensitivity: "LOW",
  vadEndSensitivity: "LOW",
  keepDirectAudio: true,
  closeToTray: true,
  proactiveSessionRefresh: false,
  transcriptLayout: "sideBySide",
  inboundVoiceOutput: "providerNative",
  outboundVoiceOutput: "providerNative",
  elevenlabsApiKeyConfigured: false,
  elevenlabsInboundVoiceId: "",
  elevenlabsInboundTtsModel: "eleven_flash_v2_5",
  elevenlabsInboundStability: 0.5,
  elevenlabsInboundSimilarityBoost: 0.75,
  elevenlabsInboundTtsSynthesisMode: "streaming",
  elevenlabsVoiceId: "",
  elevenlabsVoices: [],
  elevenlabsModels: [],
  elevenlabsTtsModel: "eleven_flash_v2_5",
  elevenlabsStability: 0.5,
  elevenlabsSimilarityBoost: 0.75,
  elevenlabsSpeed: 1.0,
  elevenlabsUseSpeakerBoost: true,
  elevenlabsChunkSchedulePreset: "fast",
  elevenlabsTtsSynthesisMode: "streaming",
  elevenlabsPlaybackCrossfade: false,
  elevenlabsCrossfadeMs: 8,
  elevenlabsTtsLanguageAuto: true,
  elevenlabsTtsLanguageCode: "",
  sonioxTtsVoices: [],
};

const translatingStatus: AppStatus = {
  running: true,
  outbound: "active",
  inbound: "direct",
  error: null,
};

describe("getPipelineModeOptions", () => {
  it("uses direction-specific labels for raw mode", () => {
    const outbound = getOutboundToolbarModeOptions(baseConfig);
    const inbound = getPipelineModeOptions("inbound");
    expect(outbound.find((o) => o.value === "originalAudio")?.label).toBe(
      "My voice (raw)",
    );
    expect(inbound.find((o) => o.value === "originalAudio")?.label).toBe(
      "Meeting (raw)",
    );
  });

  it("uses provider-aware raw tooltip copy", () => {
    const gemini = getOutboundToolbarModeOptions(baseConfig).find(
      (o) => o.value === "originalAudio",
    );
    expect(gemini?.title).toContain("Gemini");

    const soniox = getOutboundToolbarModeOptions({
      ...baseConfig,
      aiProvider: "soniox",
    }).find((o) => o.value === "originalAudio");
    expect(soniox?.title).toContain("Soniox");

    const inboundOpenAi = getPipelineModeOptions("inbound", {
      ...baseConfig,
      aiProvider: "openAi",
    }).find((o) => o.value === "originalAudio");
    expect(inboundOpenAi?.title).toContain("OpenAI");
  });

  it("disables clone options until each ElevenLabs voice is configured", () => {
    const outboundClone = getOutboundToolbarModeOptions(baseConfig).find(
      (o) => o.value === "translatedClone",
    );
    const inboundClone = getPipelineModeOptions("inbound", baseConfig).find(
      (o) => o.value === "translatedClone",
    );
    expect(outboundClone?.disabled).toBe(true);
    expect(inboundClone?.disabled).toBe(true);
    expect(inboundClone?.label).toBe("Cloned voice");
    expect(inboundClone?.shortLabel).toBe("Clone");

    const readyInboundClone = getPipelineModeOptions("inbound", {
      ...baseConfig,
      elevenlabsApiKeyConfigured: true,
      elevenlabsInboundVoiceId: "meeting-voice",
    }).find((o) => o.value === "translatedClone");
    expect(readyInboundClone?.disabled).toBe(false);
  });
});

describe("outboundToolbarModeFromConfig", () => {
  it("maps translated + clone to translatedClone", () => {
    expect(
      outboundToolbarModeFromConfig({
        ...baseConfig,
        outboundMode: "translated",
        outboundVoiceOutput: "elevenLabsClone",
      }),
    ).toBe("translatedClone");
  });
});

describe("inbound toolbar mapping", () => {
  it("maps translated clone config to and from the toolbar value", () => {
    const cloneConfig = {
      ...baseConfig,
      inboundMode: "translated" as const,
      inboundVoiceOutput: "elevenLabsClone" as const,
    };
    expect(inboundToolbarModeFromConfig(cloneConfig)).toBe("translatedClone");
    expect(inboundToolbarPatch("translatedClone", baseConfig)).toEqual({
      inboundMode: "translated",
      inboundVoiceOutput: "elevenLabsClone",
    });
    expect(inboundToolbarPatch("translated", cloneConfig)).toEqual({
      inboundMode: "translated",
      inboundVoiceOutput: "providerNative",
    });
  });
});

describe("formatOutboundToolbarShort", () => {
  it("shows Clone when clone output is active", () => {
    expect(
      formatOutboundToolbarShort({
        ...baseConfig,
        outboundMode: "translated",
        outboundVoiceOutput: "elevenLabsClone",
        elevenlabsApiKeyConfigured: true,
        elevenlabsVoiceId: "voice-1",
      }),
    ).toBe("Clone");
  });
});

describe("formatOutputModeShort", () => {
  it("returns short labels", () => {
    expect(formatOutputModeShort("textOnly", "outbound")).toBe("Captions");
    expect(formatOutputModeShort("originalAudio", "inbound")).toBe("Raw");
  });
});

describe("formatDirectionChipLabel", () => {
  it("includes output mode when translating", () => {
    expect(
      formatDirectionChipLabel(translatingStatus, baseConfig, "outbound"),
    ).toBe("Translate · Raw");
    expect(
      formatDirectionChipLabel(translatingStatus, baseConfig, "inbound"),
    ).toBe("Direct");
  });
});

describe("isOutboundCloneReady", () => {
  it("requires key and voice id", () => {
    expect(isOutboundCloneReady(baseConfig)).toBe(false);
    expect(
      isOutboundCloneReady({
        ...baseConfig,
        elevenlabsApiKeyConfigured: true,
        elevenlabsVoiceId: "  ",
      }),
    ).toBe(false);
    expect(
      isOutboundCloneReady({
        ...baseConfig,
        elevenlabsApiKeyConfigured: true,
        elevenlabsVoiceId: "voice-1",
      }),
    ).toBe(true);
  });
});

describe("isInboundCloneReady", () => {
  it("requires key and trimmed inbound voice id", () => {
    expect(isInboundCloneReady(baseConfig)).toBe(false);
    expect(
      isInboundCloneReady({
        ...baseConfig,
        elevenlabsApiKeyConfigured: true,
        elevenlabsInboundVoiceId: "  ",
      }),
    ).toBe(false);
    expect(
      isInboundCloneReady({
        ...baseConfig,
        elevenlabsApiKeyConfigured: true,
        elevenlabsInboundVoiceId: "meeting-voice",
      }),
    ).toBe(true);
  });
});
