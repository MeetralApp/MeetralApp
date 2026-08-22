import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import type { AppSnapshot } from "../lib/appState";
import { baseConfig, baseStatus } from "@/test/fixtures/config";
import { toSavePayload } from "../lib/toSavePayload";

const getAppSnapshot = vi.fn();
const saveConfig = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@/features/pipeline/api/pipelineApi", () => ({
  getAppSnapshot: (...args: unknown[]) => getAppSnapshot(...args),
  getStatus: vi.fn(),
  refreshDeviceCatalog: vi.fn(),
  setOutboundOutputMode: vi.fn(),
  setOutboundVoiceOutput: vi.fn(),
  setInboundOutputMode: vi.fn(),
  setInboundVoiceOutput: vi.fn(),
  setOutboundAudioMode: vi.fn(),
  setInboundAudioMode: vi.fn(),
  ensureDirectAudio: vi.fn(),
  setMicMuted: vi.fn(),
  setSpeakerMuted: vi.fn(),
}));

vi.mock("@/shared/lib/api/configApi", () => ({
  saveConfig: (...args: unknown[]) => saveConfig(...args),
  testApiKey: vi.fn(),
}));

vi.mock("@/shared/lib/api/voiceApi", () => ({
  testElevenLabsApiKey: vi.fn(),
  listElevenLabsVoices: vi.fn(),
  listElevenLabsModels: vi.fn(),
  listSonioxVoices: vi.fn(),
  validateElevenLabsVoice: vi.fn(),
  previewElevenLabsVoice: vi.fn(),
  previewSonioxVoice: vi.fn(),
}));

import { useTranslation } from "./useTranslation";

function sampleSnapshot(): AppSnapshot {
  return {
    revision: 1,
    runtime: { ...baseStatus, running: false },
    setup: {
      revision: 1,
      apiKeyConfigured: true,
      canDirectOutbound: true,
      canDirectInbound: true,
      canTranslateOutbound: true,
      canTranslateInbound: true,
      outboundIdleBadge: { kind: "ready", label: "Ready", title: "Ready" },
      inboundIdleBadge: { kind: "ready", label: "Ready", title: "Ready" },
      audio: { outboundReady: true, inboundReady: true, roles: [] },
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
    config: { ...baseConfig, revision: 1 },
  };
}

describe("useTranslation", () => {
  beforeEach(() => {
    getAppSnapshot.mockReset();
    saveConfig.mockReset();
    getAppSnapshot.mockResolvedValue(sampleSnapshot());
    saveConfig.mockResolvedValue({ ok: true });
  });

  it("bootstraps from app snapshot", async () => {
    const { result } = renderHook(() => useTranslation());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(getAppSnapshot).toHaveBeenCalled();
    expect(result.current.config?.aiProvider).toBe("gemini");
    expect(result.current.status?.outbound).toBe("off");
    expect(result.current.error).toBeNull();
  });

  it("saveConfig forwards payload to configApi", async () => {
    const { result } = renderHook(() => useTranslation());
    await waitFor(() => expect(result.current.loading).toBe(false));
    const payload = toSavePayload(baseConfig, { geminiApiKey: "k" });
    await act(async () => {
      await result.current.saveConfig(payload);
    });
    expect(saveConfig).toHaveBeenCalledWith(payload);
  });
});
