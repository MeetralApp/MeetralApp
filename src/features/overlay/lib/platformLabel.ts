/** Overlay UI runs inside Tauri; userAgent is enough for Cmd vs Ctrl labels. */
export const isMac =
  typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

/** Hold-to-interact key when click-through is on. */
export const overlayHoldKey = isMac ? "Cmd" : "Ctrl";

/** Global hotkey shown in Settings hints for toggling the overlay. */
export const overlayToggleHotkey = isMac ? "Cmd+Shift+O" : "Ctrl+Shift+O";

/** Global hotkey shown in Settings hints for toggling click-through. */
export const overlayClickThroughHotkey = isMac
  ? "Cmd+Shift+T"
  : "Ctrl+Shift+T";
