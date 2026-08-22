import { getCurrentWindow } from "@tauri-apps/api/window";

/** Reveal the native window after the first paint (backup to Rust on_page_load). */
export function revealAppWindow(): void {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      void getCurrentWindow()
        .show()
        .catch(() => {
        // Vite-only browser dev — no Tauri window to show.
        });
    });
  });
}
