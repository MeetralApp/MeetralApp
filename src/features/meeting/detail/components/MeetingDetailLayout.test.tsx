import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import MeetingDetailLayout from "./MeetingDetailLayout";
import type {
  MeetingRecord,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";
import { TooltipProvider } from "@/shared/ui/tooltip";

vi.mock("./ReadonlyTranscriptTimeline", () => ({
  default: () => <div data-testid="transcript-timeline" />,
}));

vi.mock("./SummaryPanel", () => ({
  default: ({ footer }: { footer?: React.ReactNode }) => (
    <div data-testid="summary-panel">{footer}</div>
  ),
}));

vi.mock("./MeetingAudioPlayer", () => ({
  default: () => <div data-testid="meeting-audio-player" />,
}));

vi.mock("./ArtifactsPanel", () => ({
  default: () => <div data-testid="artifacts-panel" />,
}));

const meeting: MeetingRecord = {
  id: "m1",
  title: "Weekly sync",
  status: "ended",
  folderId: null,
  myLanguage: "en",
  meetingLanguage: "vi",
  startedAtMs: 1_700_000_000_000,
  endedAtMs: 1_700_000_360_000,
  segmentCountOutbound: 1,
  segmentCountInbound: 1,
};

const segment: TranscriptSegment = {
  id: "seg-1",
  meetingId: "m1",
  direction: "inbound",
  sequence: 1,
  sourceText: "Hello",
  translatedText: "Xin chào",
  startedAtMs: 1_000,
  endedAtMs: 2_000,
  connectionGap: false,
};

describe("MeetingDetailLayout", () => {
  it("shows summary and transcript panes together", () => {
    render(
      <TooltipProvider>
        <MeetingDetailLayout
          meeting={meeting}
          aiProvider="gemini"
          segments={[segment]}
          summary={null}
          summaryLoading={false}
          summaryError={null}
          onGenerateSummary={vi.fn()}
          onSaveSummaryEdit={vi.fn(async () => {})}
          onDismissSummaryError={vi.fn()}
          focusedSegmentId={null}
          scrollIntent={null}
          onScrollIntentConsumed={vi.fn()}
          onFocusSegment={vi.fn()}
          onSummaryCitationClick={vi.fn()}
          onCitationClick={vi.fn()}
          seekToMs={null}
          onSeekConsumed={vi.fn()}
          onSeekFromSegment={vi.fn()}
        />
      </TooltipProvider>,
    );

    expect(screen.getByLabelText("Summary")).toBeTruthy();
    expect(screen.getByLabelText("Transcript")).toBeTruthy();
    expect(screen.getByTestId("summary-panel")).toBeTruthy();
    expect(screen.getByTestId("transcript-timeline")).toBeTruthy();
    expect(screen.getByTestId("meeting-audio-player")).toBeTruthy();
    expect(screen.getByTestId("artifacts-panel")).toBeTruthy();
  });

  it("hides Artifacts when the toggle is off", () => {
    render(
      <TooltipProvider>
        <MeetingDetailLayout
          meeting={meeting}
          aiProvider="gemini"
          segments={[]}
          summary={null}
          summaryLoading={false}
          summaryError={null}
          artifactsEnabled={false}
          onGenerateSummary={vi.fn()}
          onSaveSummaryEdit={vi.fn(async () => {})}
          onDismissSummaryError={vi.fn()}
          focusedSegmentId={null}
          scrollIntent={null}
          onScrollIntentConsumed={vi.fn()}
          onFocusSegment={vi.fn()}
          onSummaryCitationClick={vi.fn()}
          onCitationClick={vi.fn()}
          seekToMs={null}
          onSeekConsumed={vi.fn()}
          onSeekFromSegment={vi.fn()}
        />
      </TooltipProvider>,
    );

    expect(screen.queryByTestId("artifacts-panel")).toBeNull();
    expect(screen.getByTestId("summary-panel")).toBeTruthy();
  });

  it("uses Notes pane labels for notes meetings", () => {
    render(
      <TooltipProvider>
        <MeetingDetailLayout
          meeting={{ ...meeting, sessionMode: "notes" }}
          aiProvider="soniox"
          segments={[]}
          summary={null}
          summaryLoading={false}
          summaryError={null}
          onGenerateSummary={vi.fn()}
          onSaveSummaryEdit={vi.fn(async () => {})}
          onDismissSummaryError={vi.fn()}
          focusedSegmentId={null}
          scrollIntent={null}
          onScrollIntentConsumed={vi.fn()}
          onFocusSegment={vi.fn()}
          onSummaryCitationClick={vi.fn()}
          onCitationClick={vi.fn()}
          seekToMs={null}
          onSeekConsumed={vi.fn()}
          onSeekFromSegment={vi.fn()}
        />
      </TooltipProvider>,
    );

    expect(screen.getByLabelText("Meeting notes")).toBeTruthy();
    expect(screen.getByLabelText("Notes")).toBeTruthy();
  });
});
