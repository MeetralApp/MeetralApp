import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import FishAudioVoicePicker from "./FishAudioVoicePicker";

function wrap(ui: React.ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

describe("FishAudioVoicePicker", () => {
  it("shows helper copy when API key is not ready", () => {
    render(
      wrap(
        <FishAudioVoicePicker
          config={{ ...baseConfig, fishaudioApiKeyConfigured: false }}
          apiKeyReady={false}
          voicesNonce={0}
          cachedVoices={[]}
          onPersistVoices={vi.fn().mockResolvedValue(undefined)}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={vi.fn().mockResolvedValue([])}
          onToast={vi.fn()}
        />,
      ),
    );

    expect(screen.getByText(/Save your Fish Audio API key/i)).toBeTruthy();
    expect(screen.queryByLabelText("Refresh voices")).toBeNull();
  });

  it("uses cache when nonce is unchanged", async () => {
    const onListVoices = vi.fn().mockResolvedValue([]);
    const onPersistVoices = vi.fn().mockResolvedValue(undefined);

    render(
      wrap(
        <FishAudioVoicePicker
          config={{
            ...baseConfig,
            fishaudioApiKeyConfigured: true,
            fishaudioVoiceId: "voice-cached",
          }}
          apiKeyReady
          voicesNonce={0}
          cachedVoices={[{ voiceId: "voice-cached", name: "Cached Voice" }]}
          onPersistVoices={onPersistVoices}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={onListVoices}
          onToast={vi.fn()}
        />,
      ),
    );

    await waitFor(() => {
      expect(screen.getByText(/Cached Voice/)).toBeTruthy();
    });
    expect(onListVoices).not.toHaveBeenCalled();
  });

  it("refreshes when nonce changes after key save", async () => {
    const onListVoices = vi.fn().mockResolvedValue([
      { voiceId: "voice-1", name: "Fresh Voice" },
    ]);
    const onPersistVoices = vi.fn().mockResolvedValue(undefined);

    const { rerender } = render(
      wrap(
        <FishAudioVoicePicker
          config={{
            ...baseConfig,
            fishaudioApiKeyConfigured: true,
            fishaudioVoiceId: "voice-1",
          }}
          apiKeyReady
          voicesNonce={0}
          cachedVoices={[{ voiceId: "voice-1", name: "Cached Voice" }]}
          onPersistVoices={onPersistVoices}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={onListVoices}
          onToast={vi.fn()}
        />,
      ),
    );

    rerender(
      wrap(
        <FishAudioVoicePicker
          config={{
            ...baseConfig,
            fishaudioApiKeyConfigured: true,
            fishaudioVoiceId: "voice-1",
          }}
          apiKeyReady
          voicesNonce={1}
          cachedVoices={[{ voiceId: "voice-1", name: "Cached Voice" }]}
          onPersistVoices={onPersistVoices}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={onListVoices}
          onToast={vi.fn()}
        />,
      ),
    );

    await waitFor(() => {
      expect(onListVoices).toHaveBeenCalledTimes(1);
      expect(onPersistVoices).toHaveBeenCalledWith([
        { voiceId: "voice-1", name: "Fresh Voice" },
      ]);
    });
  });

  it("shows unknown voice row and empty-list field error", async () => {
    const onListVoices = vi.fn().mockResolvedValue([]);

    render(
      wrap(
        <FishAudioVoicePicker
          config={{
            ...baseConfig,
            fishaudioApiKeyConfigured: true,
            fishaudioVoiceId: "missing-voice-id-123456",
          }}
          apiKeyReady
          voicesNonce={0}
          cachedVoices={[]}
          onPersistVoices={vi.fn().mockResolvedValue(undefined)}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={onListVoices}
          onToast={vi.fn()}
        />,
      ),
    );

    await waitFor(() => {
      expect(screen.getByText(/Unknown voice/)).toBeTruthy();
      expect(screen.getByText(/No voices returned/i)).toBeTruthy();
    });
  });

  it("hint says pick the voice, not the model", () => {
    render(
      wrap(
        <FishAudioVoicePicker
          config={{ ...baseConfig, fishaudioApiKeyConfigured: true }}
          apiKeyReady
          voicesNonce={0}
          cachedVoices={[{ voiceId: "v1", name: "V1" }]}
          onPersistVoices={vi.fn().mockResolvedValue(undefined)}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={vi.fn().mockResolvedValue([])}
          onToast={vi.fn()}
        />,
      ),
    );

    expect(screen.getByLabelText("About Fish Audio voice")).toBeTruthy();
    expect(screen.queryByText(/pick the model/i)).toBeNull();
  });
});
