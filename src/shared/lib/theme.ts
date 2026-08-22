/** Theme preference + DOM application (Meetral dual theme). */

import { getCurrentWindow } from "@tauri-apps/api/window";

export type ThemePreference = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";

export const THEME_STORAGE_KEY = "meetral-theme-preference";

export const THEME_BG = {
  dark: "#0f1117",
  light: "#f4f6f8",
} as const;

export function isThemePreference(value: unknown): value is ThemePreference {
  return value === "dark" || value === "light" || value === "system";
}

export function readStoredThemePreference(): ThemePreference {
  try {
    const raw = localStorage.getItem(THEME_STORAGE_KEY);
    if (isThemePreference(raw)) return raw;
  } catch {
  /* ignore */
  }
  return "dark";
}

export function writeStoredThemePreference(preference: ThemePreference): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, preference);
  } catch {
  /* ignore */
  }
}

export function resolveTheme(
  preference: ThemePreference,
  systemDark = typeof window !== "undefined"
    ? window.matchMedia("(prefers-color-scheme: dark)").matches
    : true,
): ResolvedTheme {
  if (preference === "system") {
    return systemDark ? "dark" : "light";
  }
  return preference;
}

export function applyResolvedTheme(resolved: ResolvedTheme): void {
  const root = document.documentElement;
  if (resolved === "dark") {
    root.classList.add("dark");
  } else {
    root.classList.remove("dark");
  }
  root.style.colorScheme = resolved;
  root.style.backgroundColor = THEME_BG[resolved];
  document.body.style.backgroundColor = THEME_BG[resolved];
  document.body.style.colorScheme = resolved;

  const colorSchemeMeta = document.querySelector('meta[name="color-scheme"]');
  if (colorSchemeMeta) {
    colorSchemeMeta.setAttribute("content", resolved);
  }
  const themeColorMeta = document.querySelector('meta[name="theme-color"]');
  if (themeColorMeta) {
    themeColorMeta.setAttribute("content", THEME_BG[resolved]);
  }
}

/**
* Sync native title bar + window background with app theme.
* Skips overlay (undecorated / transparent). No-op outside Tauri.
*/
export async function syncNativeWindowChrome(
  preference: ThemePreference,
  resolved: ResolvedTheme,
): Promise<void> {
  if (typeof window === "undefined") return;
  if (window.location.hash.startsWith("#/overlay")) return;

  try {
    const win = getCurrentWindow();
    if (win.label === "overlay") return;

    await Promise.all([
      win.setTheme(preference === "system" ? null : preference),
      win.setBackgroundColor(THEME_BG[resolved]),
    ]);
  } catch {
  /* Vite browser / missing capability — DOM theme still applies */
  }
}

export function applyThemePreference(preference: ThemePreference): ResolvedTheme {
  writeStoredThemePreference(preference);
  const resolved = resolveTheme(preference);
  applyResolvedTheme(resolved);
  void syncNativeWindowChrome(preference, resolved);
  return resolved;
}
