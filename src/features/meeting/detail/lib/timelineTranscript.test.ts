import { describe, expect, it } from "vitest";

import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  audioSourceForFilter,
  filterSegmentsByDirection,
  findNearestSegmentAtTime,
  formatSegmentTimestamp,
  sortSegmentsChronologically,
  timelineFilterLabel,
  timelineSpeakerLabel,
} from "./timelineTranscript";

function seg(
  partial: Partial<TranscriptSegment> & Pick<TranscriptSegment, "id">,
): TranscriptSegment {
  return {
    meetingId: "m1",
    direction: "outbound",
    sequence: 1,
    sourceText: "",
    translatedText: "",
    startedAtMs: 0,
    endedAtMs: 0,
    connectionGap: false,
    ...partial,
  };
}

describe("sortSegmentsChronologically", () => {
  it("orders by startedAtMs then sequence", () => {
    const sorted = sortSegmentsChronologically([
      seg({ id: "c", startedAtMs: 3000, sequence: 3, direction: "inbound" }),
      seg({ id: "a", startedAtMs: 1000, sequence: 1 }),
      seg({ id: "b2", startedAtMs: 2000, sequence: 2 }),
      seg({ id: "b1", startedAtMs: 2000, sequence: 1, direction: "inbound" }),
    ]);
    expect(sorted.map((s) => s.id)).toEqual(["a", "b1", "b2", "c"]);
  });
});

describe("filterSegmentsByDirection", () => {
  const list = [
    seg({ id: "o", direction: "outbound" }),
    seg({ id: "i", direction: "inbound" }),
  ];

  it("returns all for all filter", () => {
    expect(filterSegmentsByDirection(list, "all")).toHaveLength(2);
  });

  it("filters you / meeting", () => {
    expect(filterSegmentsByDirection(list, "outbound").map((s) => s.id)).toEqual([
      "o",
    ]);
    expect(filterSegmentsByDirection(list, "inbound").map((s) => s.id)).toEqual([
      "i",
    ]);
  });
});

describe("formatSegmentTimestamp", () => {
  it("formats m:ss and h:mm:ss", () => {
    expect(formatSegmentTimestamp(0)).toBe("0:00");
    expect(formatSegmentTimestamp(65_000)).toBe("1:05");
    expect(formatSegmentTimestamp(3_725_000)).toBe("1:02:05");
  });
});

describe("timelineSpeakerLabel", () => {
  it("maps directions", () => {
    expect(timelineSpeakerLabel("outbound")).toBe("You");
    expect(timelineSpeakerLabel("inbound")).toBe("Meeting");
  });
});

describe("audioSourceForFilter", () => {
  it("maps All to room mix and speakers to solo", () => {
    expect(audioSourceForFilter("all")).toBe("room");
    expect(audioSourceForFilter("outbound")).toBe("you");
    expect(audioSourceForFilter("inbound")).toBe("meeting");
  });
});

describe("timelineFilterLabel", () => {
  it("returns All / You / Meeting", () => {
    expect(timelineFilterLabel("all")).toBe("All");
    expect(timelineFilterLabel("outbound")).toBe("You");
    expect(timelineFilterLabel("inbound")).toBe("Meeting");
  });
});

describe("findNearestSegmentAtTime", () => {
  it("returns nearest within tolerance", () => {
    const list = [
      seg({ id: "a", startedAtMs: 1000 }),
      seg({ id: "b", startedAtMs: 5000 }),
    ];
    expect(findNearestSegmentAtTime(list, 1200)?.id).toBe("a");
    expect(findNearestSegmentAtTime(list, 4800)?.id).toBe("b");
  });

  it("returns null outside tolerance", () => {
    const list = [seg({ id: "a", startedAtMs: 1000 })];
    expect(findNearestSegmentAtTime(list, 2000, 500)).toBeNull();
  });
});
