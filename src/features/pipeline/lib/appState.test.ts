import { describe, expect, it } from "vitest";
import {
  applyRevision,
  formatSetupAttentionFromSetup,
  hasSetupIssuesFromSetup,
  normalizeConfigView,
  type SetupState,
} from "./appState";
import type { AudioSetupValidation } from "@/features/audio/lib/audioSetup";
import { baseConfig } from "@/test/fixtures/config";

const readyAudio: AudioSetupValidation = {
  outboundReady: true,
  inboundReady: true,
  roles: [],
};

const readyBadge = {
  kind: "ready",
  label: "Ready",
  title: "Ready to start",
};

const readySetup: SetupState = {
  revision: 2,
  apiKeyConfigured: true,
  canDirectOutbound: true,
  canDirectInbound: true,
  canTranslateOutbound: true,
  canTranslateInbound: true,
  outboundIdleBadge: readyBadge,
  inboundIdleBadge: readyBadge,
  audio: readyAudio,
};

describe("applyRevision", () => {
  it("ignores stale events", () => {
    expect(applyRevision(readySetup, { ...readySetup, revision: 1 })).toBe(
      readySetup,
    );
  });

  it("accepts newer events", () => {
    const next = { ...readySetup, revision: 3 };
    expect(applyRevision(readySetup, next)).toEqual(next);
  });
});

describe("hasSetupIssuesFromSetup", () => {
  it("flags missing api key", () => {
    expect(
      hasSetupIssuesFromSetup({ ...readySetup, apiKeyConfigured: false }),
    ).toBe(true);
  });

  it("is false when direct paths are available", () => {
    expect(hasSetupIssuesFromSetup(readySetup)).toBe(false);
  });
});

describe("formatSetupAttentionFromSetup", () => {
  it("mentions api key when missing", () => {
    expect(
      formatSetupAttentionFromSetup({
        ...readySetup,
        apiKeyConfigured: false,
        canDirectOutbound: false,
        canDirectInbound: false,
      }),
    ).toContain("API key");
  });
});

describe("normalizeConfigView", () => {
  it("fills defaults for missing optional fields", () => {
    const normalized = normalizeConfigView({
      ...baseConfig,
      keepDirectAudio: undefined as never,
      inboundVoiceOutput: undefined,
      outboundVoiceOutput: undefined as never,
      sonioxAlwaysOn: undefined,
      elevenlabsInboundVoiceId: undefined,
      elevenlabsInboundTtsModel: undefined,
      elevenlabsInboundStability: undefined,
      elevenlabsInboundSimilarityBoost: undefined,
      elevenlabsInboundTtsSynthesisMode: undefined,
      elevenlabsVoiceId: undefined as never,
    });
    expect(normalized.keepDirectAudio).toBe(true);
    expect(normalized.inboundVoiceOutput).toBe("providerNative");
    expect(normalized.outboundVoiceOutput).toBe("providerNative");
    expect(normalized.sonioxAlwaysOn).toEqual({
      general: [],
      text: "",
      terms: [],
      translationTerms: [],
    });
    expect(normalized.elevenlabsInboundVoiceId).toBe("");
    expect(normalized.elevenlabsInboundTtsModel).toBe("eleven_flash_v2_5");
    expect(normalized.elevenlabsInboundStability).toBe(0.5);
    expect(normalized.elevenlabsInboundSimilarityBoost).toBe(0.75);
    expect(normalized.elevenlabsInboundTtsSynthesisMode).toBe("streaming");
    expect(normalized.elevenlabsVoiceId).toBe("");
    expect(normalized.sonioxEndpointLatencyAdjustmentLevel).toBe(2);
  });
});
