import { describe, expect, it } from "vitest";

import {
  resolveSettingsTab,
  type SettingsFocus,
  type SettingsTab,
} from "./settingsFocus";

/** Deep-links used by Live / banners → Settings drawer tab. */
const FOCUS_TO_TAB: [SettingsFocus | null | undefined, SettingsTab][] = [
  [null, "translate"],
  [undefined, "translate"],
  ["translate", "translate"],
  ["api", "translate"],
  ["provider", "translate"],
  ["languages", "translate"],
  ["sonioxContext", "translate"],
  ["summaries", "intelligence"],
  ["summary", "intelligence"],
  ["intelligence", "intelligence"],
  ["voice", "voice"],
  ["customVoice", "voice"],
  ["audio", "audio"],
  ["app", "app"],
  ["overlay", "app"],
];

describe("resolveSettingsTab", () => {
  it.each(FOCUS_TO_TAB)("maps %s → %s", (focus, tab) => {
    expect(resolveSettingsTab(focus)).toBe(tab);
  });

  it("routes save-related deep links to the tab that owns that section", () => {
    // Session hub "Change languages" and API banners open Translate;
    // overlay privacy / hide-from-capture opens App.
    expect(resolveSettingsTab("languages")).toBe("translate");
    expect(resolveSettingsTab("overlay")).toBe("app");
    expect(resolveSettingsTab("customVoice")).toBe("voice");
  });
});
