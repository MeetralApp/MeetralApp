import { describe, expect, it } from "vitest";
import {
  appendOverlayTail,
  clearOverlayLive,
  liveOverlayRowId,
  OVERLAY_TAIL_PER_DIRECTION,
  trimOverlayTailByDirection,
} from "./overlayTail";
import type { OverlayTailRow } from "./mockTranscript";

function row(
  partial: Partial<OverlayTailRow> & Pick<OverlayTailRow, "id" | "direction">,
): OverlayTailRow {
  return {
    sourceText: partial.sourceText ?? "s",
    translatedText: partial.translatedText ?? "t",
    live: partial.live,
    id: partial.id,
    direction: partial.direction,
  };
}

describe("appendOverlayTail", () => {
  it("updates live interim in place with a stable id", () => {
    const rows: OverlayTailRow[] = [
      {
        id: liveOverlayRowId("outbound"),
        direction: "outbound",
        sourceText: "Hel",
        translatedText: "Xin",
        live: true,
      },
    ];
    const next = appendOverlayTail(rows, {
      id: "ignored-when-live",
      direction: "outbound",
      sourceText: "Hello",
      translatedText: "Xin chào",
      live: true,
    });
    expect(next).toHaveLength(1);
    expect(next[0]?.id).toBe(liveOverlayRowId("outbound"));
    expect(next[0]?.sourceText).toBe("Hello");
    expect(next[0]?.live).toBe(true);
    expect(next[0]?.id).toBe(rows[0]?.id);
  });

  it("clears live interim when a final segment arrives", () => {
    const rows: OverlayTailRow[] = [
      {
        id: liveOverlayRowId("outbound"),
        direction: "outbound",
        sourceText: "Hello",
        translatedText: "Xin chào",
        live: true,
      },
      {
        id: "in-1",
        direction: "inbound",
        sourceText: "Hi",
        translatedText: "Chào",
      },
    ];
    const next = appendOverlayTail(rows, {
      id: "out-final",
      direction: "outbound",
      sourceText: "Hello there",
      translatedText: "Xin chào bạn",
    });
    expect(next.map((r) => r.id)).toEqual(["in-1", "out-final"]);
    expect(next.every((r) => !r.live)).toBe(true);
  });

  it("keeps live and inserts sentence commit before the live tail", () => {
    const rows: OverlayTailRow[] = [
      {
        id: liveOverlayRowId("outbound"),
        direction: "outbound",
        sourceText: "Second",
        translatedText: "Hai",
        live: true,
      },
    ];
    const next = appendOverlayTail(
      rows,
      {
        id: "seg-outbound-1",
        direction: "outbound",
        sourceText: "First.",
        translatedText: "Một.",
      },
      { clearLive: false },
    );
    expect(next.map((r) => r.id)).toEqual([
      "seg-outbound-1",
      liveOverlayRowId("outbound"),
    ]);
    expect(next[0]?.sourceText).toBe("First.");
    expect(next[1]?.sourceText).toBe("Second");
    expect(next[1]?.live).toBe(true);
  });

  it("dedupes segment-preview by stable id", () => {
    const rows: OverlayTailRow[] = [
      row({
        id: "seg-outbound-1",
        direction: "outbound",
        sourceText: "First.",
        translatedText: "Một.",
      }),
    ];
    const next = appendOverlayTail(rows, {
      id: "seg-outbound-1",
      direction: "outbound",
      sourceText: "First.",
      translatedText: "Một.",
    });
    expect(next).toHaveLength(1);
    expect(next[0]?.id).toBe("seg-outbound-1");
  });

  it("trims each direction independently", () => {
    const manyOut = Array.from({ length: OVERLAY_TAIL_PER_DIRECTION + 5 }, (_, i) =>
      row({ id: `o-${i}`, direction: "outbound", sourceText: `o${i}` }),
    );
    const fewIn = [
      row({ id: "i-0", direction: "inbound", sourceText: "i0" }),
      row({ id: "i-1", direction: "inbound", sourceText: "i1" }),
    ];
    const trimmed = trimOverlayTailByDirection([...manyOut, ...fewIn]);
    const out = trimmed.filter((r) => r.direction === "outbound");
    const inn = trimmed.filter((r) => r.direction === "inbound");
    expect(out).toHaveLength(OVERLAY_TAIL_PER_DIRECTION);
    expect(out[0]?.id).toBe("o-5");
    expect(inn.map((r) => r.id)).toEqual(["i-0", "i-1"]);
  });

  it("at capacity appends a new final without mutating previous newest", () => {
    const filled = Array.from({ length: OVERLAY_TAIL_PER_DIRECTION }, (_, i) =>
      row({
        id: `o-${i}`,
        direction: "outbound",
        sourceText: `src-${i}`,
        translatedText: `tr-${i}`,
      }),
    );
    const previousNewest = filled[filled.length - 1]!;
    const next = appendOverlayTail(filled, {
      id: "o-new",
      direction: "outbound",
      sourceText: "src-new",
      translatedText: "tr-new",
    });
    expect(next).toHaveLength(OVERLAY_TAIL_PER_DIRECTION);
    expect(next[0]?.id).toBe("o-1");
    expect(next[next.length - 1]?.id).toBe("o-new");
    expect(next[next.length - 1]?.sourceText).toBe("src-new");
    const previousInResult = next.find((r) => r.id === previousNewest.id);
    expect(previousInResult).toEqual(previousNewest);
    expect(next.some((r) => r.id === "o-0")).toBe(false);
  });
});

describe("clearOverlayLive", () => {
  it("removes only the live row for the given direction", () => {
    const rows: OverlayTailRow[] = [
      row({ id: "seg-1", direction: "outbound", sourceText: "Done" }),
      {
        id: liveOverlayRowId("outbound"),
        direction: "outbound",
        sourceText: "Live",
        translatedText: "Sống",
        live: true,
      },
      {
        id: liveOverlayRowId("inbound"),
        direction: "inbound",
        sourceText: "In",
        translatedText: "Vào",
        live: true,
      },
    ];
    const next = clearOverlayLive(rows, "outbound");
    expect(next.map((r) => r.id)).toEqual(["seg-1", liveOverlayRowId("inbound")]);
  });
});
