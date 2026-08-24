import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import XaiVoicePicker from "./XaiVoicePicker";

function wrap(ui: React.ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

describe("XaiVoicePicker", () => {
  it("shows helper copy when API key is not ready", () => {
    render(
      wrap(
        <XaiVoicePicker
          config={{ ...baseConfig, xaiApiKeyConfigured: false }}
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

    expect(screen.getByText(/Save your xAI API key/i)).toBeTruthy();
    expect(screen.queryByLabelText("Refresh voices")).toBeNull();
  });

  it("uses cache when nonce is unchanged", async () => {
    const onListVoices = vi.fn().mockResolvedValue([]);

    render(
      wrap(
        <XaiVoicePicker
          config={{
            ...baseConfig,
            xaiApiKeyConfigured: true,
            xaiVoiceId: "eve",
          }}
          apiKeyReady
          voicesNonce={0}
          cachedVoices={[{ voiceId: "eve", name: "Eve", kind: "builtIn" }]}
          onPersistVoices={vi.fn().mockResolvedValue(undefined)}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onListVoices={onListVoices}
          onToast={vi.fn()}
        />,
      ),
    );

    await waitFor(() => {
      expect(screen.getByText(/Eve/)).toBeTruthy();
    });
    expect(onListVoices).not.toHaveBeenCalled();
  });
});
