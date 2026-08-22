import type { AppStatus } from "@/shared/lib/types/pipeline";
import type { PipelineDirection } from "@/features/pipeline/lib/sessionDrift";

export type BridgeConnectionState = "idle" | "ready" | "reconnecting";

export const RECONNECT_DEBOUNCE_MS = 500;
export const RECONNECTED_BANNER_MS = 8_000;
export const RECONNECTED_COOLDOWN_MS = 2 * 60 * 1000;
export const LONG_RECONNECT_TOAST_MS = 3_000;
export const MAX_RECONNECT_ATTEMPTS = 5;

export function formatReconnectAttemptSuffix(
  attempt: number | null,
): string {
  if (attempt == null) return "";
  if (attempt <= MAX_RECONNECT_ATTEMPTS) {
    return ` (attempt ${attempt}/${MAX_RECONNECT_ATTEMPTS})`;
  }
  return ` (attempt ${attempt}, retrying…)`;
}

export function getBridgeState(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): BridgeConnectionState {
  if (!status) return "idle";
  const value =
    direction === "outbound" ? status.outboundBridge : status.inboundBridge;
  if (value === "reconnecting" || value === "ready") return value;
  return "idle";
}

export function getReconnectAttempt(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): number | null {
  if (!status) return null;
  const value =
    direction === "outbound"
      ? status.outboundReconnectAttempt
      : status.inboundReconnectAttempt;
  return value ?? null;
}

export function columnLabel(direction: PipelineDirection): string {
  return direction === "outbound" ? "You" : "Meeting";
}

export function reconnectedCooldownKey(direction: PipelineDirection): string {
  return `reconnected-last-${direction}`;
}

export function readReconnectedCooldown(
  direction: PipelineDirection,
): number | null {
  try {
    const raw = sessionStorage.getItem(reconnectedCooldownKey(direction));
    if (!raw) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function writeReconnectedCooldown(direction: PipelineDirection): void {
  try {
    sessionStorage.setItem(
      reconnectedCooldownKey(direction),
      String(Date.now()),
    );
  } catch {
  // sessionStorage unavailable
  }
}

export function canShowReconnectedBanner(
  direction: PipelineDirection,
  now: number = Date.now(),
): boolean {
  const last = readReconnectedCooldown(direction);
  if (last == null) return true;
  return now - last >= RECONNECTED_COOLDOWN_MS;
}
