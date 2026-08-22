import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { baseConfig, baseStatus } from "@/test/fixtures/config";
import { TooltipProvider } from "@/shared/ui/tooltip";
import type { ColumnUiState } from "../lib/columnUi";
import ColumnPipelineToolbar from "./ColumnPipelineToolbar";

vi.mock("@/features/voice/hooks/useVoiceTtsStatus", () => ({
  useVoiceTtsStatus: () => ({
    ready: false,
    degradedMessage: null,
  }),
}));

const readyColumnUi: ColumnUiState = {
  pipeline: "off",
  audioConnection: "ok",
  canDirect: true,
  canTranslate: true,
  translateDisabledReason: null,
  idleBadge: { kind: "ready", label: "Ready", title: "Ready" },
  pipelineLive: false,
  muteEnabled: true,
};

const modeOptions = [
  { value: "translated", label: "Translated" },
  { value: "textOnly", label: "Text only" },
];

function renderToolbar(ui: ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe("ColumnPipelineToolbar", () => {
  it("renders Direct/Translate controls and switches path", async () => {
    const onAudioPathChange = vi.fn().mockResolvedValue(undefined);
    const onModeChange = vi.fn();

    renderToolbar(
      <ColumnPipelineToolbar
        title="You"
        direction="outbound"
        status={{ ...baseStatus, outbound: "direct" }}
        config={baseConfig}
        columnUi={readyColumnUi}
        modeOptions={modeOptions}
        onAudioPathChange={onAudioPathChange}
        onModeChange={onModeChange}
        onMicMuteToggle={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("You audio path")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Direct" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Translate" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "You output mode" })).toBeTruthy();
    expect(screen.queryByText("You")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Translate" }));
    expect(onAudioPathChange).toHaveBeenCalledWith("translate");
  });

  it("shows output mode chevron while idle on Direct", () => {
    renderToolbar(
      <ColumnPipelineToolbar
        title="You"
        direction="outbound"
        status={{ ...baseStatus, outbound: "direct" }}
        config={baseConfig}
        columnUi={readyColumnUi}
        modeOptions={modeOptions}
        onAudioPathChange={vi.fn()}
        onModeChange={vi.fn()}
        onMicMuteToggle={vi.fn()}
      />,
    );

    const chevron = screen.getByRole("button", { name: "You output mode" });
    expect((chevron as HTMLButtonElement).disabled).toBe(false);
  });

  it("hot-switches mode while Translate is already active without restarting path", async () => {
    const onAudioPathChange = vi.fn().mockResolvedValue(undefined);
    const onModeChange = vi.fn();

    renderToolbar(
      <ColumnPipelineToolbar
        title="You"
        direction="outbound"
        status={{ ...baseStatus, outbound: "active" }}
        config={baseConfig}
        columnUi={{ ...readyColumnUi, pipeline: "active", pipelineLive: true }}
        modeOptions={modeOptions}
        onAudioPathChange={onAudioPathChange}
        onModeChange={onModeChange}
        onMicMuteToggle={vi.fn()}
      />,
    );

    const chevron = screen.getByRole("button", { name: "You output mode" });
    expect((chevron as HTMLButtonElement).disabled).toBe(false);
    fireEvent.pointerDown(chevron);
    fireEvent.click(chevron);

    const option = await screen.findByText("Text only");
    fireEvent.click(option);

    expect(onModeChange).toHaveBeenCalledWith("textOnly");
    expect(onAudioPathChange).not.toHaveBeenCalled();
  });

  it("starts Translate when picking a mode from Direct", async () => {
    const onAudioPathChange = vi.fn().mockResolvedValue(undefined);
    const onModeChange = vi.fn().mockResolvedValue(undefined);

    renderToolbar(
      <ColumnPipelineToolbar
        title="You"
        direction="outbound"
        status={{ ...baseStatus, outbound: "direct" }}
        config={baseConfig}
        columnUi={readyColumnUi}
        modeOptions={modeOptions}
        onAudioPathChange={onAudioPathChange}
        onModeChange={onModeChange}
        onMicMuteToggle={vi.fn()}
      />,
    );

    const chevron = screen.getByRole("button", { name: "You output mode" });
    fireEvent.pointerDown(chevron);
    fireEvent.click(chevron);

    const option = await screen.findByText("Text only");
    fireEvent.click(option);

    await vi.waitFor(() => {
      expect(onModeChange).toHaveBeenCalledWith("textOnly");
      expect(onAudioPathChange).toHaveBeenCalledWith("translate");
    });
    expect(onModeChange.mock.invocationCallOrder[0]).toBeLessThan(
      onAudioPathChange.mock.invocationCallOrder[0]!,
    );
  });

  it("disables Direct when devices are not ready", () => {
    renderToolbar(
      <ColumnPipelineToolbar
        title="Meeting"
        direction="inbound"
        status={baseStatus}
        config={baseConfig}
        columnUi={{ ...readyColumnUi, canDirect: false, canTranslate: false }}
        modeOptions={modeOptions}
        onAudioPathChange={vi.fn()}
        onModeChange={vi.fn()}
        onSpeakerMuteToggle={vi.fn()}
      />,
    );

    const direct = screen.getByRole("button", { name: "Direct" });
    const translate = screen.getByRole("button", { name: "Translate" });
    const chevron = screen.getByRole("button", { name: "Meeting output mode" });
    expect((direct as HTMLButtonElement).disabled).toBe(true);
    expect((translate as HTMLButtonElement).disabled).toBe(true);
    expect((chevron as HTMLButtonElement).disabled).toBe(true);
  });
});
