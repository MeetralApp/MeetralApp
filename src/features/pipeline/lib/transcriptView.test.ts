import { describe, expect, it } from "vitest";

import {
  buildLiveDisplayState,
  buildLiveTranscriptRows,
  buildLiveTranscriptView,
  liveDisplayRowKey,
  liveTailFingerprint,
  pruneInterimAfterSegmentCommit,
  resolveLiveTail,
  segmentToUtteranceBlock,
  snapshotFromTranscriptEvent,
  utteranceKey,
  visibleCommittedSegments,
} from "./transcriptView";
import type { TranscriptEvent } from "@/shared/lib/types/pipeline";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

function event(partial: Partial<TranscriptEvent> & Pick<TranscriptEvent, "direction">): TranscriptEvent {
  return {
    interim: false,
    turnComplete: false,
    connectionGap: false,
    ...partial,
  };
}

function segment(
  partial: Partial<TranscriptSegment> & Pick<TranscriptSegment, "id" | "direction" | "sequence">,
): TranscriptSegment {
  return {
    meetingId: "m1",
    sourceText: "",
    translatedText: "",
    startedAtMs: 0,
    endedAtMs: 0,
    connectionGap: false,
    ...partial,
  };
}

describe("snapshotFromTranscriptEvent", () => {
  it("maps backend live fields", () => {
    expect(
      snapshotFromTranscriptEvent(
        event({
          direction: "outbound",
          liveSource: "Hello",
          liveTranslated: "Xin chÃ o",
        }),
      ),
    ).toEqual({ source: "Hello", translated: "Xin chÃ o" });
  });

  it("falls back to source and translated text when live fields are absent", () => {
    expect(
      snapshotFromTranscriptEvent(
        event({
          direction: "outbound",
          sourceText: "Hello",
          translatedText: "Xin chÃ o",
          interim: true,
        }),
      ),
    ).toEqual({ source: "Hello", translated: "Xin chÃ o" });
  });

  it("returns null when live fields are empty", () => {
    expect(snapshotFromTranscriptEvent(event({ direction: "outbound" }))).toBeNull();
  });
});

describe("resolveLiveTail", () => {
  it("prefers backend live snapshot while streaming", () => {
    expect(
      resolveLiveTail({ source: "Second" }, [], []),
    ).toEqual({ source: "Second" });
  });

  it("handoffs committed tail after turnComplete until prune", () => {
    const committed = [
      segment({
        id: "client-outbound-0",
        direction: "outbound",
        sequence: 1,
        sourceText: "Done.",
      }),
    ];
    const interim = [
      event({ direction: "outbound", turnComplete: true, sourceText: "Done." }),
    ];
    expect(resolveLiveTail(null, committed, interim)?.source).toBe("Done.");
    expect(resolveLiveTail(null, committed, [])).toBeNull();
  });
});

describe("buildLiveDisplayState", () => {
  it("matches buildLiveTranscriptRows and buildLiveTranscriptView in one pass", () => {
    const committed = [
      segment({
        id: "s1",
        direction: "outbound",
        sequence: 1,
        sourceText: "First",
      }),
    ];
    const liveSnapshot = { source: "Second" };
    const interim = [event({ direction: "outbound", sourceText: "Second" })];
    const state = buildLiveDisplayState(committed, liveSnapshot, interim);
    expect(state.rows).toEqual(buildLiveTranscriptRows(committed, liveSnapshot, interim));
    expect(state.live).toEqual(buildLiveTranscriptView(committed, liveSnapshot, interim).live);
  });
});

describe("liveTailFingerprint", () => {
  it("returns null when there is no live tail", () => {
    const rows = buildLiveTranscriptRows(
      [segment({ id: "s1", direction: "outbound", sequence: 1, sourceText: "Hi" })],
      null,
      [],
    );
    expect(liveTailFingerprint(rows)).toBeNull();
  });

  it("fingerprints live tail text for scroll follow", () => {
    const rows = buildLiveTranscriptRows(
      [],
      { source: "Hello", translated: "Xin chÃ o" },
      [],
    );
    expect(liveTailFingerprint(rows)).toBe("Hello\0Xin chÃ o");
  });
});

describe("buildLiveTranscriptView", () => {
  it("returns empty when no committed or live snapshot", () => {
    expect(buildLiveTranscriptView([], null, [])).toEqual({
      history: [],
      live: null,
    });
  });

  it("maps committed segments to history", () => {
    const view = buildLiveTranscriptView(
      [segment({ id: "s1", direction: "outbound", sequence: 1, sourceText: "Hi" })],
      null,
      [],
    );
    expect(view.history).toHaveLength(1);
    expect(view.history[0]?.segmentId).toBe("s1");
    expect(view.live).toBeNull();
  });

  it("shows live from backend snapshot", () => {
    const view = buildLiveTranscriptView(
      [],
      { translated: "Live text" },
      [],
    );
    expect(view.history).toHaveLength(0);
    expect(view.live?.translated).toBe("Live text");
  });

  it("hides matching tail from history while live still shows snapshot", () => {
    const committed = [
      segment({
        id: "s1",
        direction: "outbound",
        sequence: 1,
        sourceText: "Same",
        translatedText: "Giá»‘ng",
      }),
    ];
    const view = buildLiveTranscriptView(
      committed,
      { source: "Same", translated: "Giá»‘ng" },
      [],
    );
    expect(view.history).toHaveLength(0);
    expect(view.live?.source).toBe("Same");
    expect(visibleCommittedSegments(committed, { source: "Same", translated: "Giá»‘ng" }, [])).toHaveLength(0);
  });

  it("maps connection gap segments", () => {
    const block = segmentToUtteranceBlock(
      segment({
        id: "g1",
        direction: "inbound",
        sequence: 1,
        connectionGap: true,
        sourceText: "[gap]",
      }),
    );
    expect(block.connectionGap).toBe(true);
  });
});

describe("buildLiveTranscriptRows", () => {
  it("keeps a single tail row across turnComplete with stable sequence key", () => {
    const interim = [
      event({ direction: "outbound", turnComplete: true, sourceText: "Done." }),
    ];
    const optimistic = [
      segment({
        id: "client-outbound-0",
        direction: "outbound",
        sequence: 1,
        sourceText: "Done.",
      }),
    ];
    const confirmed = [
      segment({
        id: "server-1",
        direction: "outbound",
        sequence: 1,
        sourceText: "Done.",
      }),
    ];

    const duringCommit = buildLiveTranscriptRows(optimistic, null, interim);
    expect(duringCommit).toHaveLength(1);
    expect(duringCommit[0]?.kind).toBe("live");

    const afterConfirm = buildLiveTranscriptRows(confirmed, null, []);
    expect(afterConfirm).toHaveLength(1);
    expect(afterConfirm[0]?.kind).toBe("committed");

    expect(liveDisplayRowKey(duringCommit[0]!, "outbound")).toBe(
      liveDisplayRowKey(afterConfirm[0]!, "outbound"),
    );
  });

  it("appends live tail after committed history", () => {
    const committed = [
      segment({
        id: "s1",
        direction: "outbound",
        sequence: 1,
        sourceText: "First",
      }),
    ];
    const rows = buildLiveTranscriptRows(
      committed,
      { source: "Second" },
      [],
    );
    expect(rows).toHaveLength(2);
    expect(rows[0]?.kind).toBe("committed");
    expect(rows[1]?.kind).toBe("live");
    expect(rows[1]?.kind === "live" && rows[1].block.source).toBe("Second");
    expect(rows[1]?.kind === "live" && rows[1].sequence).toBe(2);
  });

  it("dedupes Notes mirrored commit when live snapshot is source-only", () => {
    // BE Notes commit_direction sets translated=source; liveSnapshot often has source only.
    const committed = [
      segment({
        id: "s1",
        direction: "outbound",
        sequence: 1,
        sourceText: "Story thì có khi nó lại chiếm đến 95% GDP.",
        translatedText: "Story thì có khi nó lại chiếm đến 95% GDP.",
      }),
    ];
    const rows = buildLiveTranscriptRows(
      committed,
      { source: "Story thì có khi nó lại chiếm đến 95% GDP." },
      [],
    );
    expect(rows).toHaveLength(1);
    expect(rows[0]?.kind).toBe("live");
  });

  it("does not treat Interpreter source-only live as matching bilingual commit", () => {
    const committed = [
      segment({
        id: "s1",
        direction: "outbound",
        sequence: 1,
        sourceText: "Hello",
        translatedText: "Xin chào",
      }),
    ];
    const rows = buildLiveTranscriptRows(
      committed,
      { source: "Hello" },
      [],
    );
    expect(rows).toHaveLength(2);
    expect(rows[0]?.kind).toBe("committed");
    expect(rows[1]?.kind).toBe("live");
  });
});

describe("pruneInterimAfterSegmentCommit", () => {
  it("prunes interim after last turnComplete only", () => {
    const interim = [
      event({ direction: "outbound", sourceText: "Done." }),
      event({ direction: "outbound", turnComplete: true, sourceText: "Done." }),
      event({ direction: "outbound", sourceText: "Next" }),
    ];
    expect(pruneInterimAfterSegmentCommit(interim)).toEqual([
      event({ direction: "outbound", sourceText: "Next" }),
    ]);
  });
});

describe("utteranceKey", () => {
  it("is stable for identical content", () => {
    const block = { source: "Hi", translated: "ChÃ o" };
    expect(utteranceKey(block, 1)).toBe(utteranceKey(block, 1));
  });

  it("uses segmentId when present", () => {
    expect(utteranceKey({ segmentId: "seg-1" }, 0)).toBe("seg-1");
  });
});
