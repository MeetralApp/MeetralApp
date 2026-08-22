import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

const segments: TranscriptSegment[] = [
  {
    id: "db-1",
    meetingId: "m1",
    direction: "outbound",
    sequence: 1,
    sourceText: "first utterance alpha",
    translatedText: "câu đầu alpha",
    startedAtMs: 0,
    endedAtMs: 1000,
    connectionGap: false,
  },
  {
    id: "db-2",
    meetingId: "m1",
    direction: "outbound",
    sequence: 2,
    sourceText: "second utterance beta",
    translatedText: "câu hai beta",
    startedAtMs: 1000,
    endedAtMs: 2000,
    connectionGap: false,
  },
];

let committedMock: TranscriptSegment[] = segments;

vi.mock("@/features/pipeline/context/transcript/useLiveTranscript", () => ({
  useLiveCommitted: () => committedMock,
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

import { LiveTranscriptColumn } from "./LiveTranscriptColumn";

/**
* Regression: virtualized live rows must not blank older segments when a new
* one arrives. Intentionally NO TooltipProvider — virtualized rows must not
* depend on Radix Tooltip (see LiveTranscriptColumn comment).
*/
describe("LiveTranscriptColumn segment persistence", () => {
  it("keeps older committed rows visible after a new segment arrives", () => {
    committedMock = segments;
    const { rerender } = render(
      <LiveTranscriptColumn
        title="You"
        direction="outbound"
        placeholder="empty"
        transcriptLayout="sideBySide"
        banners={null}
        toolbar={<div>toolbar</div>}
      />,
    );

    expect(screen.getByText("first utterance alpha")).toBeTruthy();
    expect(screen.getByText("second utterance beta")).toBeTruthy();

    committedMock = [
      ...segments,
      {
        id: "db-3",
        meetingId: "m1",
        direction: "outbound",
        sequence: 3,
        sourceText: "third utterance gamma",
        translatedText: "câu ba gamma",
        startedAtMs: 2000,
        endedAtMs: 3000,
        connectionGap: false,
      },
    ];

    rerender(
      <LiveTranscriptColumn
        title="You"
        direction="outbound"
        placeholder="empty"
        transcriptLayout="sideBySide"
        banners={null}
        toolbar={<div>toolbar</div>}
      />,
    );

    expect(screen.getByText("first utterance alpha")).toBeTruthy();
    expect(screen.getByText("second utterance beta")).toBeTruthy();
    expect(screen.getByText("third utterance gamma")).toBeTruthy();
  });
});
