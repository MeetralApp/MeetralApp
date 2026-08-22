import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { anchorBadgeFullTitle } from "@/shared/lib/anchorBadgeStyle";
import { AnchorTimeBadge } from "./AnchorTimeBadge";

describe("AnchorTimeBadge", () => {
  it("static badge title is speaker + timestamp", () => {
    render(<AnchorTimeBadge timer="1:05" direction="inbound" speaker="Meeting" />);
    expect(screen.getByText("1:05").getAttribute("title")).toBe(
      "1:05 · Meeting",
    );
  });

  it("static badge title folds the snippet in", () => {
    render(
      <AnchorTimeBadge
        timer="0:04"
        direction="outbound"
        speaker="You"
        snippet="We approved the budget"
      />,
    );
    expect(screen.getByText("0:04").getAttribute("title")).toBe(
      "You · 0:04\nWe approved the budget",
    );
  });

  it("dot marks direction (outbound primary, inbound muted)", () => {
    const { container: out } = render(
      <AnchorTimeBadge timer="0:04" direction="outbound" />,
    );
    expect(out.querySelector(".bg-primary")).toBeTruthy();
    const { container: inb } = render(
      <AnchorTimeBadge timer="0:04" direction="inbound" />,
    );
    expect(inb.querySelector(".bg-muted-foreground\\/70")).toBeTruthy();
  });

  it("clickable badge carries jump aria-label and fires onClick", () => {
    const onClick = vi.fn();
    render(
      <TooltipProvider>
        <AnchorTimeBadge
          timer="1:05"
          direction="inbound"
          speaker="Meeting"
          onClick={onClick}
        />
      </TooltipProvider>,
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Jump to 1:05 · Meeting" }),
    );
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe("anchorBadgeFullTitle", () => {
  it("falls back to timer only, then timer · speaker, then snippet", () => {
    expect(anchorBadgeFullTitle("1:05")).toBe("1:05");
    expect(anchorBadgeFullTitle("1:05", "Meeting")).toBe("1:05 · Meeting");
    expect(anchorBadgeFullTitle("1:05", "Meeting", "Agreed")).toBe(
      "Meeting · 1:05\nAgreed",
    );
  });
});
