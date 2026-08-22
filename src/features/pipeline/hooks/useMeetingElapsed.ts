import { formatSessionElapsed, getSessionElapsedMs } from "../lib/sessionDrift";
import { useSharedClock } from "@/shared/context/useSharedClock";

/**
* Meeting-scoped elapsed time from `startedAtMs` (SQLite meeting record).
* Independent of per-column pipeline WS `activeSince` (reconnect / session refresh).
* While live, ticks with the shared clock; when `endedAtMs` is set, freezes.
*/
export function useMeetingElapsed(
  startedAtMs: number | null | undefined,
  endedAtMs: number | null | undefined = null,
): { display: string | null; title: string | null } {
  const now = useSharedClock();

  if (startedAtMs == null || !Number.isFinite(startedAtMs)) {
    return { display: null, title: null };
  }

  const endBound =
    endedAtMs != null && Number.isFinite(endedAtMs) ? endedAtMs : now;
  const elapsedMs = getSessionElapsedMs(startedAtMs, endBound);
  const display = formatSessionElapsed(elapsedMs);

  return {
    display,
    title: `Meeting · ${display}`,
  };
}
