import { describe, expect, it } from "vitest";

import {
  liveTranscriptInitialState,
  liveTranscriptReducer,
  type LiveTranscriptAction,
  type LiveTranscriptStore,
} from "./liveTranscriptReducer";
import type {
  SegmentCommittedEvent,
  SegmentPreviewEvent,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";

function preview(sequence: number, text: string): SegmentPreviewEvent {
  return {
    meetingId: "m1",
    direction: "outbound",
    sequence,
    sourceText: text,
    translatedText: text,
    connectionGap: false,
    reason: "sentence",
  };
}

function committed(id: string, sequence: number, text: string): SegmentCommittedEvent {
  const segment: TranscriptSegment = {
    id,
    meetingId: "m1",
    direction: "outbound",
    sequence,
    sourceText: text,
    translatedText: text,
    startedAtMs: 0,
    endedAtMs: 1000,
    connectionGap: false,
  };
  return { meetingId: "m1", segment };
}

function apply(state: LiveTranscriptStore, ...actions: LiveTranscriptAction[]) {
  return actions.reduce(liveTranscriptReducer, state);
}

describe("liveTranscriptReducer segment commit ordering", () => {
  it("keeps older rows when DB commits lag behind previews", () => {
    // Real BE ordering: previews emit instantly, segment-committed emits after
    // the async writer queue persists to SQLite — so preview(2) lands before
    // committed(1).
    let state = apply(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "m1",
    });
    state = apply(
      state,
      { type: "SEGMENT_PREVIEW", payload: preview(1, "first") },
      { type: "SEGMENT_PREVIEW", payload: preview(2, "second") },
    );

    expect(state.committed.outbound.map((s) => s.sequence)).toEqual([1, 2]);

    state = apply(state, {
      type: "SEGMENT_COMMITTED",
      payload: committed("db-1", 1, "first"),
    });

    // Committing seq 1 must not drop the optimistic seq-2 row nor duplicate seq 1.
    const afterFirst = state.committed.outbound;
    expect(afterFirst.map((s) => s.sequence)).toEqual([1, 2]);
    expect(new Set(afterFirst.map((s) => s.sequence)).size).toBe(2);
    expect(afterFirst[0]?.id).toBe("db-1");
    expect(afterFirst[1]?.sourceText).toBe("second");

    state = apply(state, {
      type: "SEGMENT_COMMITTED",
      payload: committed("db-2", 2, "second"),
    });

    const final = state.committed.outbound;
    expect(final.map((s) => s.id)).toEqual(["db-1", "db-2"]);
    expect(final.map((s) => s.sequence)).toEqual([1, 2]);
    expect(final.every((s) => !s.id.startsWith("client-"))).toBe(true);
  });

  it("handles interleaved preview/commit without duplicates", () => {
    let state = apply(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "m1",
    });
    state = apply(
      state,
      { type: "SEGMENT_PREVIEW", payload: preview(1, "first") },
      { type: "SEGMENT_COMMITTED", payload: committed("db-1", 1, "first") },
      { type: "SEGMENT_PREVIEW", payload: preview(2, "second") },
      { type: "SEGMENT_COMMITTED", payload: committed("db-2", 2, "second") },
    );

    const final = state.committed.outbound;
    expect(final.map((s) => s.id)).toEqual(["db-1", "db-2"]);
    expect(final.map((s) => s.sequence)).toEqual([1, 2]);
  });

  it("HYDRATE after preview does not leave duplicate sequences", () => {
    let state = apply(liveTranscriptInitialState, {
      type: "SET_ACTIVE_MEETING",
      meetingId: "m1",
    });
    state = apply(state, {
      type: "SEGMENT_PREVIEW",
      payload: preview(1, "first"),
    });
    state = apply(state, {
      type: "HYDRATE",
      meetingId: "m1",
      direction: "outbound",
      segments: [
        {
          id: "db-1",
          meetingId: "m1",
          direction: "outbound",
          sequence: 1,
          sourceText: "first",
          translatedText: "first",
          startedAtMs: 0,
          endedAtMs: 1000,
          connectionGap: false,
        },
      ],
      hasMoreOlder: false,
    });

    expect(state.committed.outbound).toHaveLength(1);
    expect(state.committed.outbound[0]?.id).toBe("db-1");

    state = apply(state, {
      type: "SEGMENT_PREVIEW",
      payload: preview(2, "second"),
    });
    const after = state.committed.outbound;
    expect(after.map((s) => s.sequence)).toEqual([1, 2]);
    expect(new Set(after.map((s) => s.sequence)).size).toBe(2);
  });
});
