import { describe, expect, it, beforeEach, afterEach } from "vitest";

import {
  applyResolvedTheme,
  resolveTheme,
  THEME_STORAGE_KEY,
} from "@/shared/lib/theme";

describe("theme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = "";
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("resolves system from media query", () => {
    expect(resolveTheme("dark")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("applies dark class and meta", () => {
    const scheme = document.createElement("meta");
    scheme.name = "color-scheme";
    document.head.appendChild(scheme);
    const theme = document.createElement("meta");
    theme.name = "theme-color";
    document.head.appendChild(theme);

    applyResolvedTheme("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(scheme.getAttribute("content")).toBe("light");
    expect(theme.getAttribute("content")).toBe("#f4f6f8");

    applyResolvedTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(scheme.getAttribute("content")).toBe("dark");
    expect(theme.getAttribute("content")).toBe("#0f1117");
  });

  it("defaults storage key constant", () => {
    expect(THEME_STORAGE_KEY).toBe("meetral-theme-preference");
  });
});
