import { describe, expect, it } from "vitest";

import {
  liveTranscriptInitialState,
  liveTranscriptReducer,
} from "./transcript/liveTranscriptReducer";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

function segment(
  id: string,
  meetingId: string,
  direction: "outbound" | "inbound",
  sequence: number,
): TranscriptSegment {
  return {
    id,
    meetingId,
    direction,
    sequence,
    sourceText: `src-${sequence}`,
    translatedText: `tr-${sequence}`,
    startedAtMs: 0,
    endedAtMs: 0,
    connectionGap: false,
  };
}

describe("liveTranscriptReducer integrity", () => {
  it("ignores HYDRATE when meetingId does not match activeMeetingId", () => {
    const state = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-b",
    });

    const next = liveTranscriptReducer(state, {
      type: "HYDRATE",
      meetingId: "meeting-a",
      direction: "outbound",
      segments: [segment("s1", "meeting-a", "outbound", 1)],
      hasMoreOlder: false,
    });

    expect(next.committed.outbound).toEqual([]);
  });

  it("ignores PREPEND_OLDER when meetingId is stale", () => {
    const withActive = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-b",
    });
    const withSegments = liveTranscriptReducer(withActive, {
      type: "HYDRATE",
      meetingId: "meeting-b",
      direction: "outbound",
      segments: [segment("s2", "meeting-b", "outbound", 2)],
      hasMoreOlder: true,
    });

    const next = liveTranscriptReducer(withSegments, {
      type: "PREPEND_OLDER",
      meetingId: "meeting-a",
      direction: "outbound",
      segments: [segment("s1", "meeting-a", "outbound", 1)],
      hasMoreOlder: false,
    });

    expect(next.committed.outbound).toHaveLength(1);
    expect(next.committed.outbound[0]?.sequence).toBe(2);
  });

  it("clears committed slice when SET_ACTIVE_MEETING changes id", () => {
    const meetingA = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const hydrated = liveTranscriptReducer(meetingA, {
      type: "HYDRATE",
      meetingId: "meeting-a",
      direction: "outbound",
      segments: [segment("s1", "meeting-a", "outbound", 1)],
      hasMoreOlder: false,
    });

    const meetingB = liveTranscriptReducer(hydrated, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-b",
    });

    expect(meetingB.activeMeetingId).toBe("meeting-b");
    expect(meetingB.committed.outbound).toEqual([]);
    expect(meetingB.committed.inbound).toEqual([]);
    expect(meetingB.interim.outbound).toEqual([]);
    expect(meetingB.liveSnapshot.outbound).toBeNull();
  });

  it("keeps SEGMENT_COMMITTED guard for wrong meetingId", () => {
    const state = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-b",
    });

    const next = liveTranscriptReducer(state, {
      type: "SEGMENT_COMMITTED",
      payload: {
        meetingId: "meeting-a",
        segment: segment("s1", "meeting-a", "outbound", 1),
      },
    });

    expect(next).toBe(state);
  });

  it("keeps SEGMENT_PREVIEW guard for wrong meetingId", () => {
    const state = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-b",
    });

    const next = liveTranscriptReducer(state, {
      type: "SEGMENT_PREVIEW",
      payload: {
        meetingId: "meeting-a",
        direction: "outbound",
        sequence: 1,
        sourceText: "hi",
        translatedText: "xin chào",
        connectionGap: false,
        reason: "turn",
      },
    });

    expect(next).toBe(state);
  });

  it("HYDRATE replaces committed segments for active meeting", () => {
    const active = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const next = liveTranscriptReducer(active, {
      type: "HYDRATE",
      meetingId: "meeting-a",
      direction: "outbound",
      segments: [
        segment("s1", "meeting-a", "outbound", 1),
        segment("s2", "meeting-a", "outbound", 2),
      ],
      hasMoreOlder: true,
    });
    expect(next.committed.outbound.map((s) => s.id)).toEqual(["s1", "s2"]);
    expect(next.committed.outboundHasMoreOlder).toBe(true);
  });

  it("SEGMENT_COMMITTED appends for matching meeting", () => {
    const active = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const hydrated = liveTranscriptReducer(active, {
      type: "HYDRATE",
      meetingId: "meeting-a",
      direction: "outbound",
      segments: [segment("s1", "meeting-a", "outbound", 1)],
      hasMoreOlder: false,
    });
    const next = liveTranscriptReducer(hydrated, {
      type: "SEGMENT_COMMITTED",
      payload: {
        meetingId: "meeting-a",
        segment: segment("s2", "meeting-a", "outbound", 2),
      },
    });
    expect(next.committed.outbound.map((s) => s.sequence)).toEqual([1, 2]);
  });

  it("TRANSCRIPT updates interim and liveSnapshot", () => {
    const active = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const next = liveTranscriptReducer(active, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "hello",
        translatedText: "xin chào",
        interim: true,
      },
    });
    expect(next.interim.outbound).toHaveLength(1);
    expect(next.liveSnapshot.outbound?.source).toBe("hello");
  });

  it("SEGMENT_PREVIEW creates optimistic then no-ops duplicate", () => {
    const active = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const withLive = liveTranscriptReducer(active, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "hi",
        translatedText: "chào",
        interim: true,
      },
    });
    expect(withLive.liveSnapshot.outbound?.source).toBe("hi");
    const previewed = liveTranscriptReducer(withLive, {
      type: "SEGMENT_PREVIEW",
      payload: {
        meetingId: "meeting-a",
        direction: "outbound",
        sequence: 1,
        sourceText: "hi",
        translatedText: "chào",
        connectionGap: false,
        reason: "turn",
      },
    });
    expect(previewed.committed.outbound[0]?.id).toMatch(/^client-/);
    expect(previewed.liveSnapshot.outbound).toBeNull();
    const dup = liveTranscriptReducer(previewed, {
      type: "SEGMENT_PREVIEW",
      payload: {
        meetingId: "meeting-a",
        direction: "outbound",
        sequence: 1,
        sourceText: "hi",
        translatedText: "chào",
        connectionGap: false,
        reason: "turn",
      },
    });
    expect(dup.committed).toBe(previewed.committed);
    expect(dup.committed.outbound).toHaveLength(1);
  });

  it("SEGMENT_PREVIEW sentence reason keeps live snapshot", () => {
    let state = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    state = liveTranscriptReducer(state, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "Still talking",
        translatedText: "Vẫn nói",
        interim: true,
      },
    });
    state = liveTranscriptReducer(state, {
      type: "SEGMENT_PREVIEW",
      payload: {
        meetingId: "meeting-a",
        direction: "outbound",
        sequence: 1,
        sourceText: "First sentence.",
        translatedText: "Câu một.",
        connectionGap: false,
        reason: "sentence",
      },
    });
    expect(state.liveSnapshot.outbound?.source).toBe("Still talking");
  });

  it("TRANSCRIPT turnComplete clears live snapshot", () => {
    let state = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    state = liveTranscriptReducer(state, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "hello",
        translatedText: "xin chào",
        interim: true,
      },
    });
    state = liveTranscriptReducer(state, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "hello",
        translatedText: "xin chào",
        interim: true,
        turnComplete: true,
      },
    });
    expect(state.liveSnapshot.outbound).toBeNull();
    expect(state.interim.outbound).toHaveLength(2);
  });

  it("SEGMENT_PREVIEW skips empty blocks", () => {
    const active = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const next = liveTranscriptReducer(active, {
      type: "SEGMENT_PREVIEW",
      payload: {
        meetingId: "meeting-a",
        direction: "outbound",
        sequence: 1,
        sourceText: "  ",
        translatedText: "",
        connectionGap: false,
        reason: "turn",
      },
    });
    expect(next.committed.outbound).toEqual([]);
  });

  it("SEGMENT_COMMITTED prunes interim", () => {
    let state = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    state = liveTranscriptReducer(state, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "partial",
        translatedText: "p",
        interim: true,
        turnComplete: true,
      },
    });
    state = liveTranscriptReducer(state, {
      type: "TRANSCRIPT",
      event: {
        direction: "outbound",
        sourceText: "after",
        translatedText: "a",
        interim: true,
      },
    });
    state = liveTranscriptReducer(state, {
      type: "SEGMENT_COMMITTED",
      payload: {
        meetingId: "meeting-a",
        segment: segment("s1", "meeting-a", "outbound", 1),
      },
    });
    expect(state.interim.outbound).toHaveLength(1);
    expect(state.interim.outbound[0]?.sourceText).toBe("after");
    expect(state.committed.outbound).toHaveLength(1);
  });

  it("SET_LOADING_OLDER CLEAR and same-meeting identity", () => {
    const active = liveTranscriptReducer(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    const same = liveTranscriptReducer(active, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "meeting-a",
    });
    expect(same).toBe(active);

    const loading = liveTranscriptReducer(active, {
      type: "SET_LOADING_OLDER",
      direction: "outbound",
      loading: true,
    });
    expect(loading.committed.loadingOlder.outbound).toBe(true);

    const cleared = liveTranscriptReducer(loading, { type: "CLEAR" });
    expect(cleared.activeMeetingId).toBeNull();
    expect(cleared.committed.outbound).toEqual([]);
  });
});
