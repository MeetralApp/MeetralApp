import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  ensureDirectAudio,
  getAppSnapshot,
  getStatus,
  previewInboundDucking,
  refreshDeviceCatalog,
  setInboundAudioMode,
  setInboundOutputMode,
  setInboundVoiceOutput,
  setMicMuted,
  setOutboundAudioMode,
  setOutboundOutputMode,
  setOutboundVoiceOutput,
  setSpeakerMuted,
} from "./pipelineApi";

describe("pipelineApi", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("getAppSnapshot and refreshDeviceCatalog pass through", async () => {
    vi.mocked(invoke).mockResolvedValue({ revision: 1 });
    await getAppSnapshot();
    expect(invoke).toHaveBeenCalledWith("get_app_snapshot");
    await refreshDeviceCatalog();
    expect(invoke).toHaveBeenCalledWith("refresh_device_catalog");
  });

  it("getStatus normalizes legacy idle", async () => {
    vi.mocked(invoke).mockResolvedValue({
      running: false,
      outbound: "idle",
      inbound: "direct",
      error: null,
    });
    const status = await getStatus();
    expect(invoke).toHaveBeenCalledWith("get_status");
    expect(status.outbound).toBe("off");
    expect(status.inbound).toBe("direct");
  });

  it("mode commands invoke expected names and normalize", async () => {
    vi.mocked(invoke).mockResolvedValue({
      running: true,
      outbound: "active",
      inbound: "off",
      error: null,
    });
    await setOutboundOutputMode("translated");
    expect(invoke).toHaveBeenCalledWith("set_outbound_output_mode", {
      mode: "translated",
    });
    await setOutboundVoiceOutput("elevenLabsClone");
    expect(invoke).toHaveBeenCalledWith("set_outbound_voice_output", {
      voiceOutput: "elevenLabsClone",
    });
    await setInboundOutputMode("textOnly");
    expect(invoke).toHaveBeenCalledWith("set_inbound_output_mode", {
      mode: "textOnly",
    });
    await setInboundVoiceOutput("elevenLabsClone");
    expect(invoke).toHaveBeenCalledWith("set_inbound_voice_output", {
      voiceOutput: "elevenLabsClone",
    });
    await setOutboundAudioMode("translate");
    expect(invoke).toHaveBeenCalledWith("set_outbound_audio_mode", {
      mode: "translate",
    });
    await setInboundAudioMode("direct");
    expect(invoke).toHaveBeenCalledWith("set_inbound_audio_mode", {
      mode: "direct",
    });
    await ensureDirectAudio();
    expect(invoke).toHaveBeenCalledWith("ensure_direct_audio");
    await setMicMuted(true);
    expect(invoke).toHaveBeenCalledWith("set_mic_muted", { muted: true });
    await setSpeakerMuted(false);
    expect(invoke).toHaveBeenCalledWith("set_speaker_muted", { muted: false });
  });

  it("previewInboundDucking invokes preview_inbound_ducking", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await previewInboundDucking({ enabled: true, gain: 0.2 });
    expect(invoke).toHaveBeenCalledWith("preview_inbound_ducking", {
      request: { enabled: true, gain: 0.2 },
    });
  });
});
