import { describe, expect, it } from "vitest";
import { overlayTranslateDisabled } from "./useOverlayRuntime";

describe("overlayTranslateDisabled", () => {
  it("disables translate while starting", () => {
    expect(
      overlayTranslateDisabled({
        pipeline: "starting",
        canTranslate: true,
        audioFault: false,
        pathMode: "direct",
      }),
    ).toBe(true);
  });

  it("disables translate while stopping", () => {
    expect(
      overlayTranslateDisabled({
        pipeline: "stopping",
        canTranslate: true,
        audioFault: false,
        pathMode: "translate",
      }),
    ).toBe(true);
  });

  it("disables translate on audio fault", () => {
    expect(
      overlayTranslateDisabled({
        pipeline: "direct",
        canTranslate: true,
        audioFault: true,
        pathMode: "direct",
      }),
    ).toBe(true);
  });

  it("allows translate when capable and idle", () => {
    expect(
      overlayTranslateDisabled({
        pipeline: "direct",
        canTranslate: true,
        audioFault: false,
        pathMode: "direct",
      }),
    ).toBe(false);
  });

  it("allows translate while direct standby is pending", () => {
    expect(
      overlayTranslateDisabled({
        pipeline: "off",
        canTranslate: true,
        audioFault: false,
        pathMode: "direct",
      }),
    ).toBe(false);
  });

  it("keeps translate enabled when already on translate path without capability", () => {
    expect(
      overlayTranslateDisabled({
        pipeline: "active",
        canTranslate: false,
        audioFault: false,
        pathMode: "translate",
      }),
    ).toBe(false);
  });
});
