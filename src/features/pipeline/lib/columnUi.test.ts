import { describe, expect, it } from "vitest";

import type { AppSnapshot } from "./appState";
import {
  selectColumnUi,
  isTranslateEnabled,
  isColumnAudioFaultFromUi,
  columnUiFromLegacy,
} from "./columnUi";

const baseSnapshot: AppSnapshot = {
  revision: 1,
  runtime: {
    running: false,
    outbound: "off",
    inbound: "off",
    error: null,
    outboundActiveSince: null,
    inboundActiveSince: null,
    outboundBridge: "idle",
    inboundBridge: "idle",
    outboundReconnectAttempt: null,
    inboundReconnectAttempt: null,
    outboundAudio: "ok",
    inboundAudio: "ok",
    outboundAudioReconnectAttempt: null,
    inboundAudioReconnectAttempt: null,
    micMuted: false,
    speakerMuted: false,
  },
  setup: {
    revision: 1,
    apiKeyConfigured: true,
    canDirectOutbound: true,
    canDirectInbound: true,
    canTranslateOutbound: true,
    canTranslateInbound: true,
    outboundIdleBadge: { kind: "ready", label: "Ready", title: "Ready" },
    inboundIdleBadge: { kind: "ready", label: "Ready", title: "Ready" },
    audio: {
      outboundReady: true,
      inboundReady: true,
      roles: [],
    },
  },
  columns: {
    outbound: {
      pipeline: "off",
      audioConnection: "ok",
      canDirect: true,
      canTranslate: true,
      translateDisabledReason: null,
      idleBadge: { kind: "ready", label: "Ready", title: "Ready" },
      pipelineLive: false,
      muteEnabled: false,
    },
    inbound: {
      pipeline: "off",
      audioConnection: "ok",
      canDirect: true,
      canTranslate: true,
      translateDisabledReason: null,
      idleBadge: { kind: "ready", label: "Ready", title: "Ready" },
      pipelineLive: false,
      muteEnabled: false,
    },
  },
  devices: { revision: 1, devices: [], enumeratedAt: 0 },
  config: {
    revision: 1,
    aiProvider: "gemini",
    apiKeyConfigured: true,
    myLanguage: "vi",
    meetingLanguage: "en",
    userMic: { id: "", name: "" },
    teamsMicFeed: { id: "", name: "" },
    meetingCapture: { id: "", name: "" },
    localPlayback: { id: "", name: "" },
    outboundMode: "translated",
    inboundMode: "translated",
    liveModel: "gemini-3.5-live-translate-preview",
    summaryModel: "gemini-2.5-flash",
    echoTargetLanguage: true,
    vadSilenceDurationMs: 500,
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
  },
};

describe("selectColumnUi", () => {
  it("returns outbound column state from snapshot", () => {
    const column = selectColumnUi(baseSnapshot, "outbound");
    expect(column?.canTranslate).toBe(true);
    expect(column?.idleBadge.kind).toBe("ready");
  });

  it("enables mute toggles when pipeline is live regardless of mute state", () => {
    const live: AppSnapshot = {
      ...baseSnapshot,
      runtime: {
        ...baseSnapshot.runtime,
        outbound: "active",
        inbound: "direct",
        micMuted: false,
        speakerMuted: true,
      },
      columns: {
        outbound: {
          ...baseSnapshot.columns.outbound,
          pipeline: "active",
          pipelineLive: true,
          muteEnabled: true,
        },
        inbound: {
          ...baseSnapshot.columns.inbound,
          pipeline: "direct",
          pipelineLive: true,
          muteEnabled: true,
        },
      },
    };
    expect(selectColumnUi(live, "outbound")?.muteEnabled).toBe(true);
    expect(selectColumnUi(live, "inbound")?.muteEnabled).toBe(true);
  });

  it("reflects audio lost consistently", () => {
    const lost: AppSnapshot = {
      ...baseSnapshot,
      columns: {
        ...baseSnapshot.columns,
        outbound: {
          ...baseSnapshot.columns.outbound,
          audioConnection: "lost",
          canTranslate: false,
          translateDisabledReason: "Audio device disconnected",
          idleBadge: {
            kind: "audio-lost",
            label: "Audio lost",
            title: "Audio device disconnected — check Settings",
          },
        },
      },
    };
    const column = selectColumnUi(lost, "outbound");
    expect(column).not.toBeNull();
    expect(isTranslateEnabled(column!)).toBe(false);
  });
});

describe("columnUi helpers", () => {
  it("isColumnAudioFaultFromUi and isTranslateEnabled", () => {
    const ok = baseSnapshot.columns.outbound;
    expect(isColumnAudioFaultFromUi(ok)).toBe(false);
    expect(isTranslateEnabled(ok)).toBe(true);
    expect(
      isColumnAudioFaultFromUi({ ...ok, audioConnection: "lost" }),
    ).toBe(true);
    expect(
      isTranslateEnabled({ ...ok, audioConnection: "reconnecting" }),
    ).toBe(false);
  });

  it("columnUiFromLegacy maps setup and status", () => {
    const column = columnUiFromLegacy(
      baseSnapshot.setup,
      {
        running: true,
        outbound: "active",
        inbound: "off",
        error: null,
        outboundAudio: "ok",
      },
      "outbound",
    );
    expect(column.pipeline).toBe("active");
    expect(column.pipelineLive).toBe(true);
    expect(column.canTranslate).toBe(true);
    expect(column.muteEnabled).toBe(true);
  });
});
