import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  TranscriptRow,
  TranscriptTableHeader,
} from "./TranscriptTable";

describe("TranscriptTable", () => {
  it("renders side-by-side header with column labels", () => {
    render(<TranscriptTableHeader layout="sideBySide" />);
    expect(screen.getByRole("columnheader", { name: /original/i })).toBeTruthy();
    expect(
      screen.getByRole("columnheader", { name: /translation/i }),
    ).toBeTruthy();
  });

  it("hides header when stacked", () => {
    const { container } = render(<TranscriptTableHeader layout="stacked" />);
    expect(container.firstChild).toBeNull();
  });

  it("stacked row puts original before translation in DOM", () => {
    render(
      <TranscriptRow
        layout="stacked"
        source="Hello"
        translated="Xin chào"
      />,
    );
    const text = screen.getByText("Hello").compareDocumentPosition(
      screen.getByText("Xin chào"),
    );
    expect(text & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("live row uses same text styles as committed history", () => {
    const { container: history } = render(
      <TranscriptRow layout="sideBySide" source="Hello" translated="Xin chào" />,
    );
    const { container: live } = render(
      <TranscriptRow
        layout="sideBySide"
        live
        source="Hello"
        translated="Xin chào"
      />,
    );

    const historySource = history.querySelector(".transcript-source");
    const liveSource = live.querySelector(".transcript-source");
    const historyTranslated = history.querySelector(".transcript-translated");
    const liveTranslated = live.querySelector(".transcript-translated");

    expect(historySource?.className).toBe(liveSource?.className);
    expect(historyTranslated?.className).toBe(liveTranslated?.className);

    const historyRow = history.firstElementChild;
    const liveRow = live.firstElementChild;
    expect(liveRow?.className).toContain("border-l-accent");
    expect(historyRow?.className).not.toContain("border-l-accent");
  });

  it("renders optional header inside the segment shell", () => {
    render(
      <TranscriptRow
        layout="stacked"
        source="Hello"
        translated="Xin chào"
        header={<span>0:13 Meeting</span>}
      />,
    );
    expect(screen.getByText("0:13 Meeting")).toBeTruthy();
    expect(screen.getByText("Hello")).toBeTruthy();
  });
});
