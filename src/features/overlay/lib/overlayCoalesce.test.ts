import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createTranscriptCoalesceScheduler } from "@/features/pipeline/context/transcript/liveTranscriptCoalesce";
import type { OverlayTailRow } from "./mockTranscript";
import {
  applyOverlayPendingBatch,
  OVERLAY_TRANSCRIPT_FLUSH_MS,
  type OverlayPending,
} from "./overlayCoalesce";

function liveRow(direction: OverlayTailRow["direction"], text: string): OverlayTailRow {
  return {
    id: `live-${direction}`,
    direction,
    sourceText: text,
    translatedText: text,
    live: true,
  };
}

function committedRow(
  direction: OverlayTailRow["direction"],
  sequence: number,
  text: string,
): OverlayTailRow {
  return {
    id: `seg-${direction}-${sequence}`,
    direction,
    sourceText: text,
    translatedText: text,
    live: false,
  };
}

describe("applyOverlayPendingBatch", () => {
  it("applies row and clearLive mutations in enqueue order", () => {
    const batch: OverlayPending[] = [
      { kind: "row", row: liveRow("outbound", "one") },
      { kind: "row", row: liveRow("inbound", "two") },
      { kind: "clearLive", direction: "outbound" },
      { kind: "row", row: liveRow("outbound", "three") },
    ];
    const next = applyOverlayPendingBatch([], batch);
    expect(next.map((r) => r.sourceText)).toEqual(["two", "three"]);
    expect(next.filter((r) => r.live && r.direction === "outbound")).toHaveLength(1);
  });

  it("committed row clears the live interim by default, keeps it when clearLive is false", () => {
    const withLive: OverlayPending[] = [{ kind: "row", row: liveRow("inbound", "draft") }];
    const base = applyOverlayPendingBatch([], withLive);

    const cleared = applyOverlayPendingBatch(base, [
      { kind: "row", row: committedRow("inbound", 1, "final") },
    ]);
    expect(cleared.map((r) => r.id)).toEqual(["seg-inbound-1"]);

    const kept = applyOverlayPendingBatch(base, [
      { kind: "row", row: committedRow("inbound", 1, "final"), clearLive: false },
    ]);
    expect(kept.map((r) => r.id)).toEqual(["seg-inbound-1", "live-inbound"]);
  });

  it("coalesced live updates keep one stable row per direction", () => {
    const batch: OverlayPending[] = [
      { kind: "row", row: liveRow("outbound", "Hel") },
      { kind: "row", row: liveRow("outbound", "Hello") },
      { kind: "row", row: liveRow("outbound", "Hello meeting") },
    ];
    const next = applyOverlayPendingBatch([], batch);
    expect(next).toHaveLength(1);
    expect(next[0].id).toBe("live-outbound");
    expect(next[0].sourceText).toBe("Hello meeting");
  });
});

describe("overlay transcript coalesce wiring (scheduler + batch flush)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("flushes a dense interim burst as one batch in order after OVERLAY_TRANSCRIPT_FLUSH_MS", () => {
    const flushed: OverlayPending[][] = [];
    const scheduler = createTranscriptCoalesceScheduler<OverlayPending>(
      (batch) => {
        flushed.push([...batch]);
      },
      OVERLAY_TRANSCRIPT_FLUSH_MS,
    );

    scheduler.enqueue({ kind: "row", row: liveRow("outbound", "a") });
    scheduler.enqueue({ kind: "row", row: liveRow("outbound", "ab") });
    scheduler.enqueue({ kind: "clearLive", direction: "inbound" });
    expect(flushed).toEqual([]);

    vi.advanceTimersByTime(OVERLAY_TRANSCRIPT_FLUSH_MS - 1);
    expect(flushed).toEqual([]);

    vi.advanceTimersByTime(1);
    expect(flushed).toHaveLength(1);
    expect(flushed[0].map((item) => item.kind)).toEqual(["row", "row", "clearLive"]);

    scheduler.dispose();
  });

  it("clear drops a queued burst without flushing (meeting switch path)", () => {
    const flushed: OverlayPending[][] = [];
    const scheduler = createTranscriptCoalesceScheduler<OverlayPending>(
      (batch) => {
        flushed.push([...batch]);
      },
      OVERLAY_TRANSCRIPT_FLUSH_MS,
    );

    scheduler.enqueue({ kind: "row", row: liveRow("outbound", "stale") });
    scheduler.clear();
    vi.advanceTimersByTime(OVERLAY_TRANSCRIPT_FLUSH_MS * 2);
    expect(flushed).toEqual([]);

    scheduler.dispose();
  });
});
