import type { AppStatus } from "@/shared/lib/types/pipeline";

export const SESSION_DRIFT_THRESHOLD_MS = 60 * 60 * 1000;

export type PipelineDirection = "outbound" | "inbound";

export function getPipelineState(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): AppStatus["outbound"] | undefined {
  if (!status) return undefined;
  return direction === "outbound" ? status.outbound : status.inbound;
}

export function getActiveSince(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): number | null {
  if (!status) return null;
  const value =
    direction === "outbound"
      ? status.outboundActiveSince
      : status.inboundActiveSince;
  return value ?? null;
}

export function isDirectionPipelineActive(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): boolean {
  if (!status) return false;
  return direction === "outbound"
    ? status.outbound === "active"
    : status.inbound === "active";
}

export interface ShouldShowSessionDriftHintInput {
  status: AppStatus | null | undefined;
  direction: PipelineDirection;
  now: number;
  dismissed: boolean;
  thresholdMs?: number;
}

export function shouldShowSessionDriftHint({
  status,
  direction,
  now,
  dismissed,
  thresholdMs = SESSION_DRIFT_THRESHOLD_MS,
}: ShouldShowSessionDriftHintInput): boolean {
  if (dismissed) return false;
  if (!isDirectionPipelineActive(status, direction)) return false;

  const since = getActiveSince(status, direction);
  if (since == null) return false;

  return now - since >= thresholdMs;
}

export function getSessionElapsedMs(since: number, now: number): number {
  return Math.max(0, now - since);
}

function pad2(value: number): string {
  return String(value).padStart(2, "0");
}

export function formatSessionElapsed(elapsedMs: number): string {
  const totalSec = Math.floor(elapsedMs / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) return `${h}:${pad2(m)}:${pad2(s)}`;
  return `${m}:${pad2(s)}`;
}

export function formatSessionElapsedTitle(elapsedMs: number): string {
  const totalSec = Math.floor(elapsedMs / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;

  const parts: string[] = [];
  if (h > 0) parts.push(`${h} hour${h === 1 ? "" : "s"}`);
  if (m > 0) parts.push(`${m} minute${m === 1 ? "" : "s"}`);
  if (h === 0 && m === 0) parts.push(`${s} second${s === 1 ? "" : "s"}`);

  return `Session running for ${parts.join(" ")}`;
}

export function isSessionPastDriftThreshold(elapsedMs: number): boolean {
  return elapsedMs >= SESSION_DRIFT_THRESHOLD_MS;
}

export function sessionDriftDismissKey(direction: PipelineDirection): string {
  return direction === "outbound"
    ? "drift-dismiss-outbound"
    : "drift-dismiss-inbound";
}

export function sessionDriftSessionKey(direction: PipelineDirection): string {
  return direction === "outbound"
    ? "drift-session-outbound"
    : "drift-session-inbound";
}

export function readDismissed(direction: PipelineDirection): boolean {
  try {
    return sessionStorage.getItem(sessionDriftDismissKey(direction)) === "1";
  } catch {
    return false;
  }
}

export function writeDismissed(
  direction: PipelineDirection,
  dismissed: boolean,
): void {
  try {
    const key = sessionDriftDismissKey(direction);
    if (dismissed) {
      sessionStorage.setItem(key, "1");
    } else {
      sessionStorage.removeItem(key);
    }
  } catch {
  // sessionStorage unavailable (private mode, etc.)
  }
}

export function clearDismissed(direction: PipelineDirection): void {
  writeDismissed(direction, false);
}

export function readStoredSessionId(
  direction: PipelineDirection,
): string | null {
  try {
    return sessionStorage.getItem(sessionDriftSessionKey(direction));
  } catch {
    return null;
  }
}

export function writeStoredSessionId(
  direction: PipelineDirection,
  sessionId: string,
): void {
  try {
    sessionStorage.setItem(sessionDriftSessionKey(direction), sessionId);
  } catch {
  // sessionStorage unavailable
  }
}

export function syncDismissForSession(
  direction: PipelineDirection,
  activeSince: number | null,
): void {
  if (activeSince == null) {
    clearDismissed(direction);
    try {
      sessionStorage.removeItem(sessionDriftSessionKey(direction));
    } catch {
    // ignore
    }
    return;
  }

  const sessionId = String(activeSince);
  const stored = readStoredSessionId(direction);
  if (stored !== sessionId) {
    clearDismissed(direction);
    writeStoredSessionId(direction, sessionId);
  }
}
