import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import LiveNotice from "./LiveNotice";

function renderNotice(ui: ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe("LiveNotice.Strip", () => {
  it("renders message and primary action", () => {
    const onClick = vi.fn();
    renderNotice(
      <LiveNotice.Strip
        message="Finish audio setup in Settings."
        actions={[{ label: "Settings", onClick }]}
      />,
    );
    expect(
      screen.getByText("Finish audio setup in Settings."),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("renders title·detail and secondary action", () => {
    renderNotice(
      <LiveNotice.Strip
        title="Playback unavailable"
        detail="Headphones (Baseus Bass BS2 Lite)"
        actions={[
          { label: "Use default", intent: "secondary", onClick: () => {} },
          { label: "Fix", onClick: () => {} },
        ]}
      />,
    );
    expect(screen.getByText("Playback unavailable")).toBeTruthy();
    expect(screen.getByText("Headphones (Baseus Bass BS2 Lite)")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Use default" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Fix" })).toBeTruthy();
  });

  it("uses alert role for destructive tone", () => {
    renderNotice(
      <LiveNotice.Strip tone="destructive" message="Connection lost." />,
    );
    expect(screen.getByRole("alert").textContent).toContain("Connection lost.");
  });
});

describe("LiveNotice.Rail", () => {
  it("renders label, meta, and dismiss", () => {
    const onDismiss = vi.fn();
    renderNotice(
      <LiveNotice.Rail
        icon="reconnecting"
        label="Reconnecting audio"
        meta="2/5"
        detail="You: audio device disconnected"
        ariaLabel="You audio device reconnecting"
        onDismiss={onDismiss}
        dismissLabel="Dismiss audio device notice"
      />,
    );
    expect(screen.getByLabelText("You audio device reconnecting")).toBeTruthy();
    expect(screen.getByText("Reconnecting audio")).toBeTruthy();
    expect(screen.getByText("2/5")).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Dismiss audio device notice" }),
    );
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
