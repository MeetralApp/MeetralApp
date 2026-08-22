import { describe, expect, it } from "vitest";

import { notesNoticeSides } from "../lib/notesNoticeSides";

describe("notesNoticeSides", () => {
  it("labels both, one, or neither side", () => {
    expect(notesNoticeSides(true, true)).toBe("You & Meeting");
    expect(notesNoticeSides(true, false)).toBe("You");
    expect(notesNoticeSides(false, true)).toBe("Meeting");
    expect(notesNoticeSides(false, false)).toBe("");
  });
});
