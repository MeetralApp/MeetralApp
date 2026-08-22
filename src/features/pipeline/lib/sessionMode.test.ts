import { describe, expect, it } from "vitest";

import { isNotesSession, notesSegmentText } from "./sessionMode";

describe("isNotesSession", () => {
  it("is true only for notes", () => {
    expect(isNotesSession({ sessionMode: "notes" })).toBe(true);
    expect(isNotesSession({ sessionMode: "interpreter" })).toBe(false);
    expect(isNotesSession({})).toBe(false);
    expect(isNotesSession(null)).toBe(false);
    expect(isNotesSession(undefined)).toBe(false);
  });
});

describe("notesSegmentText", () => {
  it("prefers source over translated", () => {
    expect(notesSegmentText("hello", "xin chào")).toBe("hello");
    expect(notesSegmentText("  hi  ", "bye")).toBe("hi");
  });

  it("falls back to translated when source empty", () => {
    expect(notesSegmentText("", "only")).toBe("only");
    expect(notesSegmentText(null, "only")).toBe("only");
    expect(notesSegmentText(undefined, "  x  ")).toBe("x");
  });

  it("returns empty when both missing", () => {
    expect(notesSegmentText("", "")).toBe("");
    expect(notesSegmentText(null, null)).toBe("");
  });
});
