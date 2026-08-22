/** Exported for unit tests — production uses via installProductionDevToolsGuards. */
export function isDevToolsShortcut(event: KeyboardEvent): boolean {
  const key = event.key;

  if (key === "F12") {
    return true;
  }

  const ctrlOrMeta = event.ctrlKey || event.metaKey;
  const shift = event.shiftKey;
  const alt = event.altKey;

  if (!ctrlOrMeta && !alt) {
    return false;
  }

  const lower = key.toLowerCase();

  // Chromium / WebView2: Ctrl+Shift+I/J/C, Ctrl+U
  if (ctrlOrMeta && shift && ["i", "j", "c"].includes(lower)) {
    return true;
  }

  // macOS: Cmd+Option+I/J/C
  if (event.metaKey && alt && ["i", "j", "c"].includes(lower)) {
    return true;
  }

  // View source
  if (ctrlOrMeta && lower === "u") {
    return true;
  }

  return false;
}

/**
* Block browser DevTools entry points in production builds only.
* Release Tauri builds already omit the devtools feature; this adds UI-level guards.
*/
export function installProductionDevToolsGuards(): void {
  if (!import.meta.env.PROD) {
    return;
  }

  document.addEventListener(
    "contextmenu",
    (event) => {
      event.preventDefault();
    },
    { capture: true },
  );

  window.addEventListener(
    "keydown",
    (event) => {
      if (isDevToolsShortcut(event)) {
        event.preventDefault();
        event.stopPropagation();
      }
    },
    { capture: true },
  );
}
