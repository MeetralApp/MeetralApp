import { describe, expect, it } from "vitest";

import {
  distanceFromBottom,
  isNearBottom,
  STICK_TO_BOTTOM_THRESHOLD_PX,
} from "./stickToBottomScroll";

function mockScrollElement({
  scrollHeight,
  clientHeight,
  scrollTop,
}: {
  scrollHeight: number;
  clientHeight: number;
  scrollTop: number;
}) {
  return {
    scrollHeight,
    clientHeight,
    scrollTop,
  } as HTMLElement;
}

describe("stickToBottomScroll", () => {
  it("returns zero distance when scrolled to bottom", () => {
    const element = mockScrollElement({
      scrollHeight: 1000,
      clientHeight: 400,
      scrollTop: 600,
    });
    expect(distanceFromBottom(element)).toBe(0);
  });

  it("detects near-bottom within threshold", () => {
    const element = mockScrollElement({
      scrollHeight: 1000,
      clientHeight: 400,
      scrollTop: 530,
    });
    expect(
      isNearBottom(element, STICK_TO_BOTTOM_THRESHOLD_PX),
    ).toBe(true);
  });

  it("detects unpinned when scrolled far from bottom", () => {
    const element = mockScrollElement({
      scrollHeight: 1000,
      clientHeight: 400,
      scrollTop: 100,
    });
    expect(
      isNearBottom(element, STICK_TO_BOTTOM_THRESHOLD_PX),
    ).toBe(false);
  });
});
