import { describe, expect, it } from "vitest";

import {
  buildTranscriptOffsets,
  estimateTranscriptRowHeight,
  estimatedScrollOffsetForIndex,
} from "./transcriptVirtualizer";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

const baseSegment: TranscriptSegment = {
  id: "s1",
  meetingId: "m1",
  direction: "outbound",
  sourceText: "Hello",
  translatedText: "Xin chÃ o",
  connectionGap: false,
  sequence: 1,
  startedAtMs: 0,
  endedAtMs: 0,
};

describe("estimateTranscriptRowHeight", () => {
  it("returns minimum height for short text", () => {
    expect(estimateTranscriptRowHeight(baseSegment)).toBe(72);
  });

  it("scales up for long paragraphs", () => {
    const longText = "word ".repeat(200);
    const height = estimateTranscriptRowHeight({
      ...baseSegment,
      sourceText: longText,
      translatedText: longText,
    });
    expect(height).toBeGreaterThan(200);
  });

  it("stacked estimate is taller than side-by-side for long text", () => {
    const longText = "word ".repeat(120);
    const segment = {
      ...baseSegment,
      sourceText: longText,
      translatedText: longText,
    };
    const sideBySide = estimateTranscriptRowHeight(segment, "sideBySide");
    const stacked = estimateTranscriptRowHeight(segment, "stacked");
    expect(stacked).toBeGreaterThan(sideBySide);
  });
});

describe("buildTranscriptOffsets", () => {
  it("returns cumulative offsets from row estimates", () => {
    const segments = [
      baseSegment,
      { ...baseSegment, id: "s2", sequence: 2 },
    ];
    const offsets = buildTranscriptOffsets(segments, "sideBySide");
    expect(offsets).toEqual([0, 72]);
  });
});

describe("estimatedScrollOffsetForIndex", () => {
  it("centers the target row in the viewport", () => {
    const segments = Array.from({ length: 10 }, (_, i) => ({
      ...baseSegment,
      id: `s${i}`,
      sequence: i + 1,
    }));
    const offsets = buildTranscriptOffsets(segments, "sideBySide");
    const offset = estimatedScrollOffsetForIndex(
      offsets,
      segments,
      5,
      "sideBySide",
      400,
      "center",
    );
    expect(offset).toBe(72 * 5 - (400 - 72) / 2);
  });

  it("clamps to list bounds", () => {
    const segments = [baseSegment];
    const offsets = buildTranscriptOffsets(segments, "sideBySide");
    expect(
      estimatedScrollOffsetForIndex(offsets, segments, 0, "sideBySide", 800, "center"),
    ).toBe(0);
  });
});
