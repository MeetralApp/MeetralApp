import type { AppStatus } from "@/shared/lib/types/pipeline";
import {
  isInboundAudioActive,
  isOutboundAudioActive,
} from "@/features/pipeline/lib/pipelineStatus";
import type { PipelineDirection } from "@/features/pipeline/lib/sessionDrift";

export type AudioConnectionState = "ok" | "reconnecting" | "lost";

export const AUDIO_RECONNECT_DEBOUNCE_MS = 500;
export const AUDIO_RECONNECTED_BANNER_MS = 8_000;
export const AUDIO_RECONNECTED_COOLDOWN_MS = 2 * 60 * 1000;
export const AUDIO_LONG_RECONNECT_TOAST_MS = 3_000;
export const MAX_AUDIO_RECONNECT_ATTEMPTS = 5;

export function formatAudioReconnectAttemptSuffix(
  attempt: number | null,
): string {
  if (attempt == null) return "";
  if (attempt <= MAX_AUDIO_RECONNECT_ATTEMPTS) {
    return ` (attempt ${attempt}/${MAX_AUDIO_RECONNECT_ATTEMPTS})`;
  }
  return ` (attempt ${attempt}, retrying…)`;
}

export function getAudioState(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): AudioConnectionState {
  if (!status) return "ok";
  const value =
    direction === "outbound" ? status.outboundAudio : status.inboundAudio;
  if (value === "reconnecting" || value === "lost") return value;
  const audioActive =
    direction === "outbound"
      ? isOutboundAudioActive(status)
      : isInboundAudioActive(status);
  if (!audioActive) return "ok";
  return "ok";
}

export function getAudioReconnectAttempt(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): number | null {
  if (!status) return null;
  const value =
    direction === "outbound"
      ? status.outboundAudioReconnectAttempt
      : status.inboundAudioReconnectAttempt;
  return value ?? null;
}

export function audioReconnectedCooldownKey(
  direction: PipelineDirection,
): string {
  return `audio-reconnected-last-${direction}`;
}

export function readAudioReconnectedCooldown(
  direction: PipelineDirection,
): number | null {
  try {
    const raw = sessionStorage.getItem(audioReconnectedCooldownKey(direction));
    if (!raw) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function writeAudioReconnectedCooldown(
  direction: PipelineDirection,
): void {
  try {
    sessionStorage.setItem(
      audioReconnectedCooldownKey(direction),
      String(Date.now()),
    );
  } catch {
  // sessionStorage unavailable
  }
}

export function canShowAudioReconnectedBanner(
  direction: PipelineDirection,
  now: number = Date.now(),
): boolean {
  const last = readAudioReconnectedCooldown(direction);
  if (last == null) return true;
  return now - last >= AUDIO_RECONNECTED_COOLDOWN_MS;
}

export function columnLabel(direction: PipelineDirection): string {
  return direction === "outbound" ? "You" : "Meeting";
}

export function isColumnAudioFault(
  status: AppStatus | null | undefined,
  direction: PipelineDirection,
): boolean {
  const audio = getAudioState(status, direction);
  return audio === "reconnecting" || audio === "lost";
}

export function isAnyColumnAudioFault(
  status: AppStatus | null | undefined,
): boolean {
  return (
    isColumnAudioFault(status, "outbound") ||
    isColumnAudioFault(status, "inbound")
  );
}

export function areAllAudioPathsOk(
  status: AppStatus | null | undefined,
): boolean {
  if (!status) return true;
  return (
    getAudioState(status, "outbound") === "ok" &&
    getAudioState(status, "inbound") === "ok"
  );
}
