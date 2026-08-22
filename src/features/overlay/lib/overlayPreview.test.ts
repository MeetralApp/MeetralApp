import { describe, expect, it } from "vitest";

import { shouldWipeOverlayTailOnPreviewEvent } from "./overlayPreview";

describe("shouldWipeOverlayTailOnPreviewEvent", () => {
  it("wipes when entering preview", () => {
    expect(shouldWipeOverlayTailOnPreviewEvent(false, true)).toBe(true);
  });

  it("wipes when leaving preview", () => {
    expect(shouldWipeOverlayTailOnPreviewEvent(true, false)).toBe(true);
  });

  it("does not wipe a live tail on redundant preview=false (Rescue/show)", () => {
    expect(shouldWipeOverlayTailOnPreviewEvent(false, false)).toBe(false);
  });

  it("keeps wipe when re-entering preview while already preview", () => {
    expect(shouldWipeOverlayTailOnPreviewEvent(true, true)).toBe(true);
  });
});
