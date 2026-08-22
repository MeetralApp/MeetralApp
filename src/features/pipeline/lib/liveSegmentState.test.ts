import { describe, expect, it } from "vitest";

import {
  createOptimisticSegment,
  dedupeSegmentsById,
  isClientSegmentId,
  mergeHydratedSegments,
  peekNextSequence,
  prependOlderSegments,
  insertSegmentSorted,
  replaceOptimisticOrAppend,
  sortSegmentsAsc,
} from "./liveSegmentState";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

function segment(
  id: string,
  sequence: number,
  direction: "outbound" | "inbound" = "outbound",
): TranscriptSegment {
  return {
    id,
    meetingId: "m1",
    direction,
    sequence,
    sourceText: "a",
    translatedText: "b",
    startedAtMs: 0,
    endedAtMs: 0,
    connectionGap: false,
  };
}

describe("insertSegmentSorted", () => {
  it("inserts in sequence order", () => {
    const result = insertSegmentSorted(
      [segment("s2", 2), segment("s4", 4)],
      segment("s3", 3),
    );
    expect(result.map((s) => s.sequence)).toEqual([2, 3, 4]);
  });

  it("replaces by id", () => {
    const result = insertSegmentSorted(
      [segment("s1", 1)],
      { ...segment("s1", 1), sourceText: "updated" },
    );
    expect(result[0]?.sourceText).toBe("updated");
  });
});

describe("replaceOptimisticOrAppend", () => {
  it("matches dedupe+append behavior for new server segment", () => {
    const committed = [segment("s1", 1), segment("client-outbound-0", 2)];
    const server = segment("db-2", 2);
    const result = replaceOptimisticOrAppend(committed, server);
    expect(result).toHaveLength(2);
    expect(result[1]?.id).toBe("db-2");
  });

  it("replaces the optimistic with the same sequence, not the newest one", () => {
    const committed = [
      segment("client-outbound-0", 1),
      segment("client-outbound-1", 2),
    ];
    const result = replaceOptimisticOrAppend(committed, segment("db-1", 1));
    expect(result.map((s) => s.id)).toEqual(["db-1", "client-outbound-1"]);
    expect(result.map((s) => s.sequence)).toEqual([1, 2]);
  });

  it("appends in order when no optimistic matches the sequence", () => {
    const committed = [segment("s1", 1)];
    const result = replaceOptimisticOrAppend(committed, segment("db-2", 2));
    expect(result.map((s) => s.id)).toEqual(["s1", "db-2"]);
  });
});

describe("id and sequence helpers", () => {
  it("isClientSegmentId", () => {
    expect(isClientSegmentId("client-outbound-1")).toBe(true);
    expect(isClientSegmentId("db-1")).toBe(false);
  });

  it("peekNextSequence", () => {
    expect(peekNextSequence([])).toBe(1);
    expect(peekNextSequence([segment("s1", 1), segment("s3", 3)])).toBe(4);
  });

  it("sortSegmentsAsc and dedupeSegmentsById", () => {
    expect(
      sortSegmentsAsc([segment("s2", 2), segment("s1", 1)]).map((s) => s.id),
    ).toEqual(["s1", "s2"]);
    expect(
      dedupeSegmentsById([
        segment("s1", 1),
        { ...segment("s1", 1), sourceText: "newer" },
        segment("s2", 2),
      ]).map((s) => s.sourceText),
    ).toEqual(["newer", "a"]);
  });
});

describe("hydrate merge helpers", () => {
  it("mergeHydratedSegments and prependOlderSegments", () => {
    expect(
      mergeHydratedSegments([segment("s2", 2)], [segment("s1", 1)]).map(
        (s) => s.sequence,
      ),
    ).toEqual([1, 2]);
    expect(
      prependOlderSegments([segment("s3", 3)], [segment("s1", 1), segment("s2", 2)]).map(
        (s) => s.sequence,
      ),
    ).toEqual([1, 2, 3]);
  });

  it("mergeHydratedSegments collapses optimistic+server same sequence (virtualizer key race)", () => {
    // Preview lands before hydrate returns the already-persisted row → two ids,
    // one sequence. liveDisplayRowKey is sequence-only, so duplicates blank older
    // rows when the virtualizer reshuffles on the next append.
    const optimistic = segment("client-outbound-0", 1);
    const server = { ...segment("db-1", 1), sourceText: "from-db" };
    const merged = mergeHydratedSegments([optimistic], [server]);
    expect(merged).toHaveLength(1);
    expect(merged[0]?.id).toBe("db-1");
    expect(merged.map((s) => s.sequence)).toEqual([1]);
  });

  it("replaceOptimisticOrAppend collapses stray same-sequence duplicates", () => {
    const committed = [
      segment("client-outbound-0", 1),
      segment("db-1", 1),
      segment("client-outbound-1", 2),
    ];
    const result = replaceOptimisticOrAppend(committed, segment("db-2", 2));
    expect(result.map((s) => s.sequence)).toEqual([1, 2]);
    expect(result.map((s) => s.id)).toEqual(["db-1", "db-2"]);
  });

  it("createOptimisticSegment", () => {
    const opt = createOptimisticSegment(
      "inbound",
      4,
      "m1",
      9,
      "hi",
      "chào",
      false,
    );
    expect(opt.id).toBe("client-inbound-4");
    expect(opt.sequence).toBe(9);
    expect(opt.direction).toBe("inbound");
  });
});
