import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import SonioxEngineVoiceSettings from "./SonioxEngineVoiceSettings";

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

const ttsModels = [
  { id: "tts-rt-v1", name: "v1", languages: [] },
  { id: "tts-rt-v2", name: "v2", languages: [] },
];

describe("SonioxEngineVoiceSettings", () => {
  it("shows the shared TTS model on the You → Meeting Engine column", () => {
    render(
      wrap(
        <SonioxEngineVoiceSettings
          config={{ ...baseConfig, sonioxTtsModel: "tts-rt-v1" }}
          direction="outbound"
          locked={false}
          ttsModels={ttsModels}
          sonioxVoices={[{ id: "Adrian", name: "Adrian", gender: "male" }]}
          catalogLoading={false}
          showSharedModel
          onRefreshCatalog={vi.fn()}
          onSave={vi.fn().mockResolvedValue(undefined)}
          onToast={vi.fn()}
        />,
      ),
    );
    expect(screen.getByText("TTS model")).toBeTruthy();
    expect(document.getElementById("soniox-tts-outbound-model")).toBeTruthy();
  });

  it("saves a model pick without refreshing the catalog", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onRefreshCatalog = vi.fn();
    render(
      wrap(
        <SonioxEngineVoiceSettings
          config={{ ...baseConfig, sonioxTtsModel: "tts-rt-v1" }}
          direction="inbound"
          locked={false}
          ttsModels={ttsModels}
          sonioxVoices={[{ id: "Adrian", name: "Adrian", gender: "male" }]}
          catalogLoading={false}
          showSharedModel
          onRefreshCatalog={onRefreshCatalog}
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

    expect(onSave).toHaveBeenCalledWith({ sonioxTtsModel: "tts-rt-v2" });
    expect(onRefreshCatalog).not.toHaveBeenCalled();
  });
});
