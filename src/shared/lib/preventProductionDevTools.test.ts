import { describe, expect, it } from "vitest";
import { isDevToolsShortcut } from "./preventProductionDevTools";

function keyEvent(
  key: string,
  mods: Partial<Pick<KeyboardEvent, "ctrlKey" | "metaKey" | "shiftKey" | "altKey">> = {},
): KeyboardEvent {
  return {
    key,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ...mods,
  } as KeyboardEvent;
}

describe("isDevToolsShortcut", () => {
  it("detects F12 and common inspector chords", () => {
    expect(isDevToolsShortcut(keyEvent("F12"))).toBe(true);
    expect(
      isDevToolsShortcut(keyEvent("I", { ctrlKey: true, shiftKey: true })),
    ).toBe(true);
    expect(
      isDevToolsShortcut(keyEvent("j", { ctrlKey: true, shiftKey: true })),
    ).toBe(true);
    expect(
      isDevToolsShortcut(keyEvent("C", { metaKey: true, shiftKey: true })),
    ).toBe(true);
    expect(isDevToolsShortcut(keyEvent("u", { ctrlKey: true }))).toBe(true);
    expect(
      isDevToolsShortcut(keyEvent("i", { metaKey: true, altKey: true })),
    ).toBe(true);
  });

  it("ignores unrelated keys", () => {
    expect(isDevToolsShortcut(keyEvent("a"))).toBe(false);
    expect(isDevToolsShortcut(keyEvent("i", { ctrlKey: true }))).toBe(false);
    expect(isDevToolsShortcut(keyEvent("Enter", { ctrlKey: true }))).toBe(
      false,
    );
  });
});
