import type { OverlayTailRow } from "./mockTranscript";

/** Max committed+live rows kept per direction (You / Meeting). */
export const OVERLAY_TAIL_PER_DIRECTION = 12;

/** Stable id for the in-flight interim row per direction (avoids remount storms). */
export function liveOverlayRowId(direction: OverlayTailRow["direction"]): string {
  return `live-${direction}`;
}

/** Drop oldest rows per direction while preserving mixed chronological order. */
export function trimOverlayTailByDirection(
  rows: OverlayTailRow[],
  perDirection = OVERLAY_TAIL_PER_DIRECTION,
): OverlayTailRow[] {
  let outbound = 0;
  let inbound = 0;
  for (const row of rows) {
    if (row.direction === "inbound") inbound += 1;
    else outbound += 1;
  }
  if (outbound <= perDirection && inbound <= perDirection) return rows;

  let dropOutbound = Math.max(0, outbound - perDirection);
  let dropInbound = Math.max(0, inbound - perDirection);
  return rows.filter((row) => {
    if (row.direction === "inbound") {
      if (dropInbound > 0) {
        dropInbound -= 1;
        return false;
      }
      return true;
    }
    if (dropOutbound > 0) {
      dropOutbound -= 1;
      return false;
    }
    return true;
  });
}

/** Drop the in-flight live row for a direction (turn end / gap). */
export function clearOverlayLive(
  rows: OverlayTailRow[],
  direction: OverlayTailRow["direction"],
): OverlayTailRow[] {
  return rows.filter((row) => !(row.live && row.direction === direction));
}

export type AppendOverlayTailOptions = {
  /**
  * When true (default), remove the live interim for this direction.
  * Sentence-boundary commits must keep live so the next utterance stays visible.
  */
  clearLive?: boolean;
};

/**
* Append or update the overlay transcript tail.
* Live interim rows keep a stable id and are updated in place so React does not
* remount the DOM on every STT token (critical for overlay CPU).
*
* Committed rows come from `segment-preview`. Pass `clearLive: false` for
* mid-turn sentence commits so the live tail is preserved and stays after the
* new committed row (same order as Live's buildLiveDisplayState).
*/
export function appendOverlayTail(
  rows: OverlayTailRow[],
  next: OverlayTailRow,
  options?: AppendOverlayTailOptions,
): OverlayTailRow[] {
  if (next.live) {
    const id = liveOverlayRowId(next.direction);
    const idx = rows.findIndex(
      (row) => row.live && row.direction === next.direction,
    );
    const liveRow: OverlayTailRow = { ...next, id, live: true };
    if (idx >= 0) {
      const copy = rows.slice();
      copy[idx] = liveRow;
      return copy;
    }
    return trimOverlayTailByDirection([...rows, liveRow]);
  }

  const clearLive = options?.clearLive ?? true;
  const withoutDup = rows.filter((row) => row.id !== next.id);
  const committed: OverlayTailRow = { ...next, live: false };

  if (clearLive) {
    const filtered = withoutDup.filter(
      (row) => !(row.live && row.direction === next.direction),
    );
    return trimOverlayTailByDirection([...filtered, committed]);
  }

  // Keep live: insert committed before the live row for this direction.
  const liveIdx = withoutDup.findIndex(
    (row) => row.live && row.direction === next.direction,
  );
  if (liveIdx >= 0) {
    const copy = withoutDup.slice();
    copy.splice(liveIdx, 0, committed);
    return trimOverlayTailByDirection(copy);
  }
  return trimOverlayTailByDirection([...withoutDup, committed]);
}

export function filterOverlayRows(
  rows: OverlayTailRow[],
  inbound: boolean,
  outbound: boolean,
) {
  return rows.filter((row) =>
    row.direction === "inbound" ? inbound : outbound,
  );
}
