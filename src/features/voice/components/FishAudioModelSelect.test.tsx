import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import FishAudioModelSelect from "./FishAudioModelSelect";

function wrap(ui: React.ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

describe("FishAudioModelSelect", () => {
  it("uses cache when nonce is unchanged", async () => {
    const onListModels = vi.fn().mockResolvedValue([
      { modelId: "s2.1-pro", name: "S2.1 Pro" },
    ]);
    const onPersistModels = vi.fn().mockResolvedValue(undefined);

    render(
      wrap(
        <FishAudioModelSelect
          value="s2.1-pro"
          onValueChange={vi.fn()}
          apiKeyReady
          refreshNonce={0}
          cachedModels={[{ modelId: "s2.1-pro", name: "Cached Pro" }]}
          onPersistModels={onPersistModels}
          onListModels={onListModels}
        />,
      ),
    );

    await waitFor(() => {
      expect(screen.getByText("Cached Pro")).toBeTruthy();
    });
    expect(onListModels).not.toHaveBeenCalled();
    expect(onPersistModels).not.toHaveBeenCalled();
  });

  it("refreshes and persists when nonce changes", async () => {
    const onListModels = vi.fn().mockResolvedValue([
      { modelId: "s2.1-pro", name: "S2.1 Pro" },
      { modelId: "s2.1-pro-free", name: "S2.1 Pro Free" },
    ]);
    const onPersistModels = vi.fn().mockResolvedValue(undefined);

    const { rerender } = render(
      wrap(
        <FishAudioModelSelect
          value="s2.1-pro"
          onValueChange={vi.fn()}
          apiKeyReady
          refreshNonce={0}
          cachedModels={[{ modelId: "s2.1-pro", name: "Cached Pro" }]}
          onPersistModels={onPersistModels}
          onListModels={onListModels}
        />,
      ),
    );

    rerender(
      wrap(
        <FishAudioModelSelect
          value="s2.1-pro"
          onValueChange={vi.fn()}
          apiKeyReady
          refreshNonce={1}
          cachedModels={[{ modelId: "s2.1-pro", name: "Cached Pro" }]}
          onPersistModels={onPersistModels}
          onListModels={onListModels}
        />,
      ),
    );

    await waitFor(() => {
      expect(onListModels).toHaveBeenCalledTimes(1);
      expect(onPersistModels).toHaveBeenCalledWith([
        { modelId: "s2.1-pro", name: "S2.1 Pro" },
        { modelId: "s2.1-pro-free", name: "S2.1 Pro Free" },
      ]);
    });
  });

  it("shows unknown saved model and empty-list error", async () => {
    const onListModels = vi.fn().mockResolvedValue([]);
    const onPersistModels = vi.fn().mockResolvedValue(undefined);

    render(
      wrap(
        <FishAudioModelSelect
          value="legacy-model"
          onValueChange={vi.fn()}
          apiKeyReady
          refreshNonce={0}
          cachedModels={[]}
          onPersistModels={onPersistModels}
          onListModels={onListModels}
        />,
      ),
    );

    await waitFor(() => {
      expect(screen.getByText(/Unknown model/)).toBeTruthy();
      expect(
        screen.getByText(/No TTS models returned/),
      ).toBeTruthy();
    });
  });

  it("manual refresh reloads the allow-list", async () => {
    const onListModels = vi
      .fn()
      .mockResolvedValueOnce([{ modelId: "s2.1-pro", name: "S2.1 Pro" }])
      .mockResolvedValueOnce([
        { modelId: "s2.1-pro", name: "S2.1 Pro" },
        { modelId: "s2.1-pro-free", name: "S2.1 Pro Free" },
      ]);
    const onPersistModels = vi.fn().mockResolvedValue(undefined);

    render(
      wrap(
        <FishAudioModelSelect
          value="s2.1-pro"
          onValueChange={vi.fn()}
          apiKeyReady
          refreshNonce={0}
          cachedModels={[]}
          onPersistModels={onPersistModels}
          onListModels={onListModels}
        />,
      ),
    );

    const refresh = await screen.findByRole("button", { name: "Refresh models" });
    expect(onListModels).toHaveBeenCalledTimes(1);
    expect((refresh as HTMLButtonElement).disabled).toBe(false);

    fireEvent.click(refresh);

    await waitFor(() => {
      expect(onListModels).toHaveBeenCalledTimes(2);
      expect(onPersistModels).toHaveBeenCalledTimes(2);
    });
  });
});
