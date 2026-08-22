import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import { LiveTranscriptColumn } from "./LiveTranscriptColumn";

vi.mock("@/features/pipeline/context/transcript/useLiveTranscript", () => ({
  useLiveCommitted: () => [],
  useLiveInterim: () => [],
  useLiveSnapshot: () => null,
  useLiveHasMoreOlder: () => false,
  useLiveLoadingOlder: () => false,
}));

vi.mock("../hooks/useLiveSegmentPagination", () => ({
  useLiveSegmentPagination: () => ({
    loadOlder: vi.fn(),
    loadingOlder: false,
    hasMoreOlder: false,
  }),
}));

vi.mock("@/shared/hooks/useStickToBottomScroll", () => ({
  useStickToBottomScroll: () => ({
    isPinned: true,
    newSinceUnpinned: 0,
    jumpToBottom: vi.fn(),
    releasePin: vi.fn(),
    beginProgrammaticScroll: vi.fn(),
  }),
}));

describe("LiveTranscriptColumn", () => {
  it("renders empty placeholder", () => {
    render(
      <LiveTranscriptColumn
        title="You"
        direction="outbound"
        placeholder="Your speech and translation appear here."
        transcriptLayout="sideBySide"
        banners={null}
        toolbar={<div>You toolbar</div>}
      />,
    );

    expect(
      screen.getByText("Your speech and translation appear here."),
    ).toBeTruthy();
    expect(screen.getByText("You toolbar")).toBeTruthy();
  });
});
