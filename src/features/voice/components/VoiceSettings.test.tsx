import type { ComponentProps, ReactNode } from "react";
import { describe, expect, it, vi, afterEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import type { ConfigView } from "@/shared/lib/types/pipeline";
import * as voiceApi from "@/shared/lib/api/voiceApi";
import * as aiApi from "@/features/ai/lib/aiApi";
import VoiceSettings from "./VoiceSettings";
import FishAudioCustomVoicePanel from "./FishAudioCustomVoicePanel";

vi.mock("@/shared/lib/api/voiceApi", () => ({
  listElevenLabsVoices: vi.fn().mockResolvedValue([]),
  listElevenLabsModels: vi.fn().mockResolvedValue([]),
  listSonioxVoices: vi.fn().mockResolvedValue([]),
  validateElevenLabsVoice: vi.fn().mockResolvedValue(undefined),
  previewElevenLabsVoice: vi.fn().mockResolvedValue(undefined),
  previewSonioxVoice: vi.fn().mockResolvedValue(undefined),
  testElevenLabsApiKey: vi.fn().mockResolvedValue(undefined),
  testFishAudioApiKey: vi.fn().mockResolvedValue(undefined),
  listFishAudioVoices: vi.fn().mockResolvedValue([]),
  listFishAudioModels: vi.fn().mockResolvedValue([]),
  validateFishAudioVoice: vi.fn().mockResolvedValue(undefined),
  previewFishAudioVoice: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/features/ai/lib/aiApi", () => ({
  listSonioxTtsModels: vi.fn().mockResolvedValue([]),
}));

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

function renderVoice(config: ConfigView = baseConfig) {
  const props: ComponentProps<typeof VoiceSettings> = {
    config,
    outboundLocked: false,
    inboundLocked: false,
    onSave: vi.fn().mockResolvedValue(undefined),
    onToast: vi.fn(),
  };
  return render(wrap(<VoiceSettings {...props} />));
}

describe("VoiceSettings", () => {
  afterEach(() => {
    vi.mocked(voiceApi.listFishAudioVoices).mockResolvedValue([]);
    vi.mocked(voiceApi.listFishAudioModels).mockResolvedValue([]);
    vi.mocked(voiceApi.listSonioxVoices).mockResolvedValue([]);
    vi.mocked(aiApi.listSonioxTtsModels).mockResolvedValue([]);
  });
  it("renders engine voice without crashing", () => {
    renderVoice();
    expect(screen.getByText("Meeting → You")).toBeTruthy();
    expect(screen.getByText("You → Meeting")).toBeTruthy();
    expect(screen.getByLabelText("What you hear")).toBeTruthy();
    expect(screen.getByLabelText("How you sound")).toBeTruthy();
  });

  it("renders custom voice without crashing", () => {
    renderVoice({
      ...baseConfig,
      inboundVoiceOutput: "custom",
      outboundVoiceOutput: "custom",
    });
    expect(screen.getAllByText("Custom voice engine").length).toBeGreaterThan(0);
    expect(screen.getAllByText("ElevenLabs").length).toBeGreaterThan(0);
  });

  it("renders custom voice + ElevenLabs without crashing", () => {
    renderVoice({
      ...baseConfig,
      inboundVoiceOutput: "custom",
      outboundVoiceOutput: "custom",
      inboundCustomVoiceVendor: "elevenLabs",
      outboundCustomVoiceVendor: "elevenLabs",
      elevenlabsApiKeyConfigured: true,
    });
    expect(screen.getAllByText("Custom voice engine").length).toBe(2);
  });

  it("renders custom voice + Fish Audio with empty model catalog without crashing", async () => {
    renderVoice({
      ...baseConfig,
      inboundVoiceOutput: "custom",
      outboundVoiceOutput: "custom",
      inboundCustomVoiceVendor: "fishAudio",
      outboundCustomVoiceVendor: "fishAudio",
      fishaudioApiKeyConfigured: true,
    });
    expect(screen.getAllByText("Fish Audio").length).toBeGreaterThanOrEqual(2);
    await waitFor(() => {
      expect(screen.getAllByLabelText("Refresh voices").length).toBe(2);
      expect(screen.getAllByLabelText("Preview voice").length).toBe(2);
    });
  });

  it("renders when Fish fields are missing from get_config", () => {
    const {
      outboundCustomVoiceVendor: _outVendor,
      inboundCustomVoiceVendor: _inVendor,
      fishaudioApiKeyConfigured: _key,
      fishaudioVoiceId: _voice,
      fishaudioInboundVoiceId: _inVoice,
      fishaudioVoices: _voices,
      fishaudioModels: _models,
      fishaudioTtsModel: _model,
      fishaudioInboundTtsModel: _inModel,
      fishaudioLatency: _lat,
      fishaudioInboundLatency: _inLat,
      fishaudioTemperature: _temp,
      fishaudioInboundTemperature: _inTemp,
      fishaudioSpeed: _speed,
      fishaudioTopP: _topP,
      ...legacy
    } = baseConfig;
    renderVoice(legacy);
    expect(screen.getByText("Meeting → You")).toBeTruthy();
  });

  it("does not overwrite Fish Audio vendor when the voice catalog persists", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      wrap(
        <VoiceSettings
          config={{
            ...baseConfig,
            inboundVoiceOutput: "custom",
            outboundVoiceOutput: "custom",
            inboundCustomVoiceVendor: "elevenLabs",
            outboundCustomVoiceVendor: "elevenLabs",
            fishaudioApiKeyConfigured: true,
          }}
          outboundLocked={false}
          inboundLocked={false}
          onSave={onSave}
          onToast={vi.fn()}
        />,
      ),
    );

    const trigger = document.getElementById("outbound-custom-voice-vendor");
    fireEvent.keyDown(trigger as HTMLElement, { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("option", { name: "Fish Audio" }));

    await waitFor(() => {
      expect(onSave.mock.calls.length).toBeGreaterThanOrEqual(1);
    });
    await waitFor(() => {
      const vendors = onSave.mock.calls.map((call) => {
        const payload = call[0] as { outboundCustomVoiceVendor?: string };
        return payload.outboundCustomVoiceVendor;
      });
      expect(vendors[vendors.length - 1]).toBe("fishAudio");
      expect(vendors.every((vendor) => vendor === "fishAudio")).toBe(true);
    });
    expect(document.getElementById("outbound-custom-voice-vendor")?.textContent).toContain(
      "Fish Audio",
    );
  });

  it("keeps Engine voice after a delayed custom voice catalog persist", async () => {
    let resolveVoices!: (list: { voiceId: string; name: string }[]) => void;
    vi.mocked(voiceApi.listFishAudioVoices).mockReturnValue(
      new Promise((resolve) => {
        resolveVoices = resolve;
      }),
    );
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      wrap(
        <VoiceSettings
          config={{
            ...baseConfig,
            inboundVoiceOutput: "custom",
            outboundVoiceOutput: "custom",
            inboundCustomVoiceVendor: "fishAudio",
            outboundCustomVoiceVendor: "fishAudio",
            fishaudioApiKeyConfigured: true,
          }}
          outboundLocked={false}
          inboundLocked={false}
          onSave={onSave}
          onToast={vi.fn()}
        />,
      ),
    );

    const trigger = document.getElementById("voice-output-mode");
    fireEvent.keyDown(trigger as HTMLElement, { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("option", { name: "Engine voice" }));

    await waitFor(() => {
      const lastCall = onSave.mock.calls[onSave.mock.calls.length - 1];
      const last = lastCall?.[0] as {
        outboundVoiceOutput?: string;
        interpreterOutboundVoiceOutput?: string;
      };
      expect(last.outboundVoiceOutput).toBe("providerNative");
      expect(last.interpreterOutboundVoiceOutput).toBe("providerNative");
    });

    resolveVoices([{ voiceId: "voice-1", name: "Voice 1" }]);

    await waitFor(() => {
      expect(
        onSave.mock.calls.some((call) => {
          const payload = call[0] as { fishaudioVoices?: unknown[] };
          return (payload.fishaudioVoices?.length ?? 0) > 0;
        }),
      ).toBe(true);
    });

    const lastCall = onSave.mock.calls[onSave.mock.calls.length - 1];
    const last = lastCall?.[0] as {
      outboundVoiceOutput?: string;
      interpreterOutboundVoiceOutput?: string;
    };
    expect(last.outboundVoiceOutput).toBe("providerNative");
    expect(last.interpreterOutboundVoiceOutput).toBe("providerNative");
    expect(document.getElementById("voice-output-mode")?.textContent).toContain(
      "Engine voice",
    );
  });

  it("keeps Engine voice after Fish model catalog persist from Advanced", async () => {
    vi.mocked(voiceApi.listFishAudioModels).mockResolvedValue([
      { modelId: "s2.1-pro", name: "S2.1 Pro" },
      { modelId: "s2.1-pro-free", name: "S2.1 Pro Free" },
    ]);
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      wrap(
        <VoiceSettings
          config={{
            ...baseConfig,
            inboundVoiceOutput: "custom",
            outboundVoiceOutput: "custom",
            inboundCustomVoiceVendor: "fishAudio",
            outboundCustomVoiceVendor: "fishAudio",
            fishaudioApiKeyConfigured: true,
          }}
          outboundLocked={false}
          inboundLocked={false}
          onSave={onSave}
          onToast={vi.fn()}
        />,
      ),
    );

    const trigger = document.getElementById("voice-output-mode");
    fireEvent.keyDown(trigger as HTMLElement, { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("option", { name: "Engine voice" }));

    await waitFor(() => {
      const lastCall = onSave.mock.calls[onSave.mock.calls.length - 1];
      const last = lastCall?.[0] as { outboundVoiceOutput?: string };
      expect(last.outboundVoiceOutput).toBe("providerNative");
    });

    fireEvent.click(screen.getAllByText("Advanced")[0]!);

    await waitFor(() => {
      expect(
        onSave.mock.calls.some((call) => {
          const payload = call[0] as { fishaudioModels?: unknown[] };
          return (payload.fishaudioModels?.length ?? 0) > 0;
        }),
      ).toBe(true);
    });

    const lastCall = onSave.mock.calls[onSave.mock.calls.length - 1];
    const last = lastCall?.[0] as {
      outboundVoiceOutput?: string;
      interpreterOutboundVoiceOutput?: string;
    };
    expect(last.outboundVoiceOutput).toBe("providerNative");
    expect(last.interpreterOutboundVoiceOutput).toBe("providerNative");
  });

  it("shows the shared Soniox TTS model on both Engine columns", () => {
    renderVoice({
      ...baseConfig,
      aiProvider: "soniox",
      sonioxApiKeyConfigured: true,
      sonioxTtsModel: "tts-rt-v1",
      sonioxTtsModels: [
        { id: "tts-rt-v1", name: "v1", languages: [] },
        { id: "tts-rt-v2", name: "v2", languages: [] },
      ],
      sonioxTtsVoices: [{ id: "Adrian", name: "Adrian", gender: "male" }],
    });
    expect(screen.getAllByText("TTS model")).toHaveLength(2);
    expect(document.getElementById("soniox-tts-inbound-model")).toBeTruthy();
    expect(document.getElementById("soniox-tts-outbound-model")).toBeTruthy();
  });

  it("does not persist the previous Soniox model when the catalog refreshes", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    let resolveModels!: (models: { id: string; name: string; languages: [] }[]) => void;
    let resolveVoices!: (voices: { id: string; name: string; gender: string }[]) => void;
    vi.mocked(aiApi.listSonioxTtsModels).mockReturnValue(
      new Promise((resolve) => {
        resolveModels = resolve;
      }),
    );
    vi.mocked(voiceApi.listSonioxVoices).mockReturnValue(
      new Promise((resolve) => {
        resolveVoices = resolve;
      }),
    );

    render(
      wrap(
        <VoiceSettings
          config={{
            ...baseConfig,
            aiProvider: "soniox",
            sonioxApiKeyConfigured: true,
            sonioxTtsModel: "tts-rt-v1",
            sonioxTtsModels: [
              { id: "tts-rt-v1", name: "v1", languages: [] },
              { id: "tts-rt-v2", name: "v2", languages: [] },
            ],
            sonioxTtsVoices: [{ id: "Adrian", name: "Adrian", gender: "male" }],
          }}
          outboundLocked={false}
          inboundLocked={false}
          onSave={onSave}
          onToast={vi.fn()}
        />,
      ),
    );

    fireEvent.keyDown(
      document.getElementById("soniox-tts-inbound-model") as HTMLElement,
      { key: "ArrowDown" },
    );
    fireEvent.click(await screen.findByRole("option", { name: "v2" }));

    await waitFor(() => {
      expect(
        onSave.mock.calls.some((call) => {
          const payload = call[0] as { sonioxTtsModel?: string };
          return payload.sonioxTtsModel === "tts-rt-v2";
        }),
      ).toBe(true);
    });

    const callsBeforeCatalog = onSave.mock.calls.length;
    fireEvent.click(screen.getAllByLabelText("Refresh voices")[0]);
    resolveModels([
      { id: "tts-rt-v1", name: "v1", languages: [] },
      { id: "tts-rt-v2", name: "v2", languages: [] },
    ]);
    resolveVoices([{ id: "Adrian", name: "Adrian", gender: "male" }]);

    await waitFor(() => {
      expect(onSave.mock.calls.length).toBeGreaterThan(callsBeforeCatalog);
    });

    const catalogSaves = onSave.mock.calls.slice(callsBeforeCatalog);
    for (const [payload] of catalogSaves) {
      expect(payload.sonioxTtsModel).not.toBe("tts-rt-v1");
      expect(
        payload.sonioxTtsModel === undefined ||
          payload.sonioxTtsModel === "tts-rt-v2",
      ).toBe(true);
    }
  });
});

describe("FishAudioCustomVoicePanel", () => {
  it("does not crash when Advanced is opened before models load", async () => {
    const customVoiceSectionRef = { current: null };
    const onListFishAudioModels = vi.fn().mockResolvedValue([
      { modelId: "s2.1-pro", name: "S2.1 Pro" },
    ]);
    const persistFishAudioModels = vi.fn().mockResolvedValue(undefined);
    render(
      wrap(
        <FishAudioCustomVoicePanel
          config={{ ...baseConfig, fishaudioApiKeyConfigured: true }}
          direction="outbound"
          locked={false}
          customVoiceSectionRef={customVoiceSectionRef}
          voicesNonce={0}
          ttsModel="s2.1-pro"
          setTtsModel={vi.fn()}
          latency="balanced"
          setLatency={vi.fn()}
          temperature={0.7}
          setTemperature={vi.fn()}
          speed={1}
          setSpeed={vi.fn()}
          topP={0.7}
          setTopP={vi.fn()}
          customVoiceSettingsDirty={false}
          customVoiceSettingsSaving={false}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onTestFishAudio={vi.fn().mockResolvedValue(undefined)}
          onListFishAudioVoices={vi.fn().mockResolvedValue([])}
          onListFishAudioModels={onListFishAudioModels}
          onToast={vi.fn()}
          onKeyDirty={vi.fn()}
          onVoiceDirty={vi.fn()}
          onKeySaved={vi.fn()}
          persistFishAudioVoices={vi.fn().mockResolvedValue(undefined)}
          persistFishAudioModels={persistFishAudioModels}
          resetCustomVoiceSettings={vi.fn()}
          persistCustomVoiceSettings={vi.fn()}
        />,
      ),
    );

    fireEvent.click(screen.getByText("Advanced"));
    expect(screen.getByLabelText("TTS model")).toBeTruthy();
    expect(screen.getByLabelText("Latency")).toBeTruthy();
    await waitFor(() => {
      expect(screen.getByLabelText("Refresh models")).toBeTruthy();
      expect(onListFishAudioModels).toHaveBeenCalled();
      expect(persistFishAudioModels).toHaveBeenCalled();
    });
  });
});
