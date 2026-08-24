import { describe, expect, it } from "vitest";
import {
  audioSectionStatus,
  audioTabWarn,
  dirtySectionStatus,
  intelligenceSectionStatus,
  intelligenceTabWarn,
  translateTabWarn,
  translationSectionStatus,
  voiceSectionStatus,
  voiceTabWarn,
} from "./settingsDrawerStatus";
import { baseConfig } from "@/test/fixtures/config";
import type { AudioSetupValidation } from "@/features/audio/lib/audioSetup";

const readyAudio: AudioSetupValidation = {
  outboundReady: true,
  inboundReady: true,
  roles: [],
};

describe("translationSectionStatus", () => {
  it("warns when api key missing or dirty", () => {
    expect(
      translationSectionStatus({ ...baseConfig, apiKeyConfigured: false }, false),
    ).toEqual({ tone: "warn", label: "Setup needed" });
    expect(translationSectionStatus(baseConfig, true)).toEqual({
      tone: "warn",
      label: "Not saved",
    });
    expect(translationSectionStatus(baseConfig, false)).toEqual({
      tone: "ok",
      label: "Ready",
    });
  });
});

describe("intelligenceSectionStatus", () => {
  it("checks summary provider key flags", () => {
    expect(
      intelligenceSectionStatus({
        ...baseConfig,
        summaryProvider: "gemini",
        geminiApiKeyConfigured: true,
      }),
    ).toEqual({ tone: "ok", label: "Ready" });
    expect(
      intelligenceSectionStatus({
        ...baseConfig,
        summaryProvider: "openAi",
        openaiApiKeyConfigured: false,
      }),
    ).toEqual({ tone: "warn", label: "Setup needed" });
  });
});

describe("voiceSectionStatus", () => {
  it("reports engine voice when not cloning", () => {
    expect(voiceSectionStatus(baseConfig, false)).toEqual({
      tone: "ok",
      label: "Engine voice",
    });
  });

  it("warns when custom voice setup incomplete", () => {
    expect(
      voiceSectionStatus(
        {
          ...baseConfig,
          outboundVoiceOutput: "custom",
          elevenlabsApiKeyConfigured: false,
          elevenlabsVoiceId: "",
        },
        false,
      ),
    ).toEqual({ tone: "warn", label: "Setup needed" });
  });

  it("checks inbound custom voice readiness independently", () => {
    const inboundCustom = {
      ...baseConfig,
      inboundVoiceOutput: "custom" as const,
      elevenlabsApiKeyConfigured: true,
      elevenlabsInboundVoiceId: "inbound-v1",
    };
    expect(voiceSectionStatus(inboundCustom, false)).toEqual({
      tone: "ok",
      label: "Custom voice ready",
    });
    expect(
      voiceSectionStatus(
        { ...inboundCustom, elevenlabsInboundVoiceId: "" },
        false,
      ),
    ).toEqual({ tone: "warn", label: "Setup needed" });
  });
});

describe("audioSectionStatus and dirtySectionStatus", () => {
  it("marks dirty audio as not saved", () => {
    expect(audioSectionStatus(true, readyAudio)).toEqual({
      tone: "warn",
      label: "Not saved",
    });
    expect(dirtySectionStatus(true)).toEqual({
      tone: "warn",
      label: "Not saved",
    });
    expect(dirtySectionStatus(false)).toBeUndefined();
  });
});

describe("tab warn dots", () => {
  it("translateTabWarn follows key, soniox dirty, and speech detection dirty", () => {
    expect(translateTabWarn(baseConfig, false, false)).toBe(false);
    expect(translateTabWarn(baseConfig, true, false)).toBe(true);
    expect(
      translateTabWarn({ ...baseConfig, aiProvider: "soniox" }, false, true),
    ).toBe(true);
    expect(translateTabWarn(baseConfig, false, false, true)).toBe(true);
  });

  it("intelligenceTabWarn and voiceTabWarn and audioTabWarn", () => {
    expect(
      intelligenceTabWarn({
        ...baseConfig,
        summaryProvider: "gemini",
        geminiApiKeyConfigured: false,
      }),
    ).toBe(true);
    expect(
      voiceTabWarn(
        {
          ...baseConfig,
          outboundVoiceOutput: "custom",
          elevenlabsApiKeyConfigured: true,
          elevenlabsVoiceId: "v1",
        },
        false,
      ),
    ).toBe(false);
    expect(
      voiceTabWarn(
        {
          ...baseConfig,
          inboundVoiceOutput: "custom",
          elevenlabsApiKeyConfigured: true,
          elevenlabsInboundVoiceId: "inbound-v1",
        },
        false,
      ),
    ).toBe(false);
    expect(
      voiceTabWarn(
        {
          ...baseConfig,
          inboundVoiceOutput: "custom",
          elevenlabsApiKeyConfigured: true,
          elevenlabsInboundVoiceId: "",
        },
        false,
      ),
    ).toBe(true);
    expect(audioTabWarn(true, readyAudio)).toBe(true);
  });
});
