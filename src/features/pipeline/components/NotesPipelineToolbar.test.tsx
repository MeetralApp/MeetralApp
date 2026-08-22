import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { baseConfig, baseStatus } from "@/test/fixtures/config";
import { TooltipProvider } from "@/shared/ui/tooltip";
import type { ColumnUiState } from "../lib/columnUi";
import NotesPipelineToolbar from "./NotesPipelineToolbar";

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

function renderToolbar(ui: ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe("NotesPipelineToolbar", () => {
  it("renders one Direct/Notes cluster and toggles both paths", async () => {
    const onOutbound = vi.fn().mockResolvedValue(undefined);
    const onInbound = vi.fn().mockResolvedValue(undefined);

    renderToolbar(
      <NotesPipelineToolbar
        status={{ ...baseStatus, outbound: "direct", inbound: "direct" }}
        config={{ ...baseConfig, sessionMode: "notes" }}
        outboundColumn={readyColumnUi}
        inboundColumn={readyColumnUi}
        onOutboundAudioPathChange={onOutbound}
        onInboundAudioPathChange={onInbound}
        onMicMuteToggle={vi.fn()}
        onSpeakerMuteToggle={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("Notes audio path")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Direct" })).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: "Notes" })).toHaveLength(1);

    fireEvent.click(screen.getByRole("button", { name: "Notes" }));
    expect(onOutbound).toHaveBeenCalledWith("translate");
    expect(onInbound).toHaveBeenCalledWith("translate");
  });

  it("still stops both paths when one side rejects", async () => {
    const onOutbound = vi.fn().mockRejectedValue(new Error("outbound failed"));
    const onInbound = vi.fn().mockResolvedValue(undefined);

    renderToolbar(
      <NotesPipelineToolbar
        status={{ ...baseStatus, outbound: "active", inbound: "active" }}
        config={{ ...baseConfig, sessionMode: "notes" }}
        outboundColumn={{ ...readyColumnUi, pipeline: "active" }}
        inboundColumn={{ ...readyColumnUi, pipeline: "active" }}
        onOutboundAudioPathChange={onOutbound}
        onInboundAudioPathChange={onInbound}
        onMicMuteToggle={vi.fn()}
        onSpeakerMuteToggle={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Direct" }));
    await vi.waitFor(() => {
      expect(onOutbound).toHaveBeenCalledWith("direct");
      expect(onInbound).toHaveBeenCalledWith("direct");
    });
  });

  it("anchors mic left, Direct/Notes center, speaker right", () => {
    renderToolbar(
      <NotesPipelineToolbar
        status={baseStatus}
        config={{ ...baseConfig, sessionMode: "notes" }}
        outboundColumn={readyColumnUi}
        inboundColumn={readyColumnUi}
        onOutboundAudioPathChange={vi.fn()}
        onInboundAudioPathChange={vi.fn()}
        micMuted={false}
        speakerMuted={false}
        onMicMuteToggle={vi.fn()}
        onSpeakerMuteToggle={vi.fn()}
      />,
    );

    const rail = screen.getByLabelText("Notes audio path");
    const buttons = rail.querySelectorAll("button");
    const names = [...buttons].map(
      (btn) => btn.getAttribute("aria-label") ?? btn.textContent?.trim() ?? "",
    );

    expect(names[0]).toMatch(/Mute voice in meeting/i);
    expect(names).toContain("Direct");
    expect(names).toContain("Notes");
    expect(names).toContain("Mute meeting audio");

    const micIndex = names.findIndex((n) => /Mute voice in meeting/i.test(n));
    const directIndex = names.indexOf("Direct");
    const notesIndex = names.indexOf("Notes");
    const speakerIndex = names.findIndex((n) => /Mute meeting audio/i.test(n));
    expect(micIndex).toBeLessThan(directIndex);
    expect(directIndex).toBeLessThan(notesIndex);
    expect(notesIndex).toBeLessThan(speakerIndex);
  });
});
