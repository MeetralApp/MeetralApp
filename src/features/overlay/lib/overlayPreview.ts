/**
* Overlay preview-mode event handling (FE).
*
* `overlay::show` / Rescue / Shift+K used to emit `overlay-preview-mode: false`
* even when already live — the listener wiped the live transcript tail. Only
* wipe when entering preview (seed mock) or leaving preview (drop mock).
*/
export function shouldWipeOverlayTailOnPreviewEvent(
  currentlyPreview: boolean,
  nextPreview: boolean,
): boolean {
  if (nextPreview) return true;
  return currentlyPreview;
}
