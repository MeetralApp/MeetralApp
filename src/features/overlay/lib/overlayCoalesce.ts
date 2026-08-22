import type { OverlayTailRow } from "./mockTranscript";
import { appendOverlayTail, clearOverlayLive } from "./overlayTail";

/** Cap React commits during dense STT interim bursts (long meetings). */
export const OVERLAY_TRANSCRIPT_FLUSH_MS = 80;

/** Pending UI mutations coalesced into one React commit every OVERLAY_TRANSCRIPT_FLUSH_MS. */
export type OverlayPending =
  | { kind: "row"; row: OverlayTailRow; clearLive?: boolean }
  | { kind: "clearLive"; direction: OverlayTailRow["direction"] };

/**
* Pure batch application — applies queued mutations in order against the tail.
* Kept separate from the scheduler so the coalesce semantics are testable
* without React or timers.
*/
export function applyOverlayPendingBatch(
  prev: OverlayTailRow[],
  batch: OverlayPending[],
): OverlayTailRow[] {
  let next = prev;
  for (const item of batch) {
    if (item.kind === "clearLive") {
      next = clearOverlayLive(next, item.direction);
    } else {
      next = appendOverlayTail(next, item.row, { clearLive: item.clearLive });
    }
  }
  return next;
}
