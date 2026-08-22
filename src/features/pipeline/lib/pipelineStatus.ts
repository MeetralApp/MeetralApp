import type {
  AppStatus,
  AudioPathMode,
  ConfigView,
  PipelineOutputMode,
  PipelineState,
} from "@/shared/lib/types/pipeline";

export function normalizePipelineOutputMode(
  mode: string,
): PipelineOutputMode {
  if (mode === "originalAudio") return "originalAudio";
  if (mode === "textOnly") return "textOnly";
  return "translated";
}

export function needsPlaybackMode(mode: PipelineOutputMode): boolean {
  return mode === "translated" || mode === "originalAudio";
}

export function normalizePipelineState(state: string): PipelineState {
  if (state === "idle") return "off";
  if (
    state === "off" ||
    state === "direct" ||
    state === "starting" ||
    state === "stopping" ||
    state === "active" ||
    state === "error"
  ) {
    return state;
  }
  return "off";
}

export function normalizeAppStatus(status: AppStatus): AppStatus {
  return {
    ...status,
    outbound: normalizePipelineState(status.outbound),
    inbound: normalizePipelineState(status.inbound),
    outboundAudio: status.outboundAudio ?? "ok",
    inboundAudio: status.inboundAudio ?? "ok",
    micMuted: status.micMuted ?? false,
    speakerMuted: status.speakerMuted ?? false,
  };
}

export function isDirectionTranslating(
  status: AppStatus | null | undefined,
  direction: "outbound" | "inbound",
): boolean {
  if (!status) return false;
  const state =
    direction === "outbound" ? status.outbound : status.inbound;
  return state === "starting" || state === "stopping" || state === "active";
}

export function isDirectionDirect(
  status: AppStatus | null | undefined,
  direction: "outbound" | "inbound",
): boolean {
  if (!status) return false;
  const state =
    direction === "outbound" ? status.outbound : status.inbound;
  return state === "direct";
}

export function getAudioPathMode(
  status: AppStatus | null | undefined,
  direction: "outbound" | "inbound",
): AudioPathMode {
  return isDirectionTranslating(status, direction) ? "translate" : "direct";
}

export function isPipelineBusy(status: AppStatus | null | undefined): boolean {
  if (!status) return false;
  return (
    status.outbound === "starting" ||
    status.outbound === "stopping" ||
    status.outbound === "active" ||
    status.inbound === "starting" ||
    status.inbound === "stopping" ||
    status.inbound === "active"
  );
}

export interface NewMeetingLock {
  locked: boolean;
  hint?: string;
}

/** Lock + New meeting while live toolbar controls are unavailable (starting / stopping / direct standby). */
export function getNewMeetingLock(
  status: AppStatus | null | undefined,
  config: ConfigView,
  canDirectOutbound: boolean,
  canDirectInbound: boolean,
): NewMeetingLock {
  if (!status) return { locked: false };

  if (status.outbound === "starting" || status.inbound === "starting") {
    return {
      locked: true,
      hint: "Wait until translation finishes connecting",
    };
  }

  if (status.outbound === "stopping" || status.inbound === "stopping") {
    return {
      locked: true,
      hint: "Wait until translation finishes stopping",
    };
  }

  if (
    isColumnAwaitingDirectStandby(
      config,
      status,
      "outbound",
      canDirectOutbound,
    ) ||
    isColumnAwaitingDirectStandby(config, status, "inbound", canDirectInbound)
  ) {
    return { locked: true, hint: "Starting direct audio…" };
  }

  return { locked: false };
}

export function isOutboundAudioActive(
  status: AppStatus | null | undefined,
): boolean {
  if (!status) return false;
  const state = status.outbound;
  return (
    state === "direct" ||
    state === "starting" ||
    state === "stopping" ||
    state === "active"
  );
}

export function isInboundAudioActive(
  status: AppStatus | null | undefined,
): boolean {
  if (!status) return false;
  const state = status.inbound;
  return (
    state === "direct" ||
    state === "starting" ||
    state === "stopping" ||
    state === "active"
  );
}

/** Pipeline is running (direct, starting, or translate active) for a column. */
export function isColumnPipelineLive(
  status: AppStatus | null | undefined,
  direction: "outbound" | "inbound",
): boolean {
  return direction === "outbound"
    ? isOutboundAudioActive(status)
    : isInboundAudioActive(status);
}

/** Direct standby is configured but the column pipeline has not restarted yet. */
export function isColumnAwaitingDirectStandby(
  config: ConfigView,
  status: AppStatus | null | undefined,
  direction: "outbound" | "inbound",
  canDirect: boolean,
): boolean {
  return (
    config.keepDirectAudio &&
    canDirect &&
    !isColumnPipelineLive(status, direction)
  );
}

export function isDirectionActive(
  status: AppStatus | null | undefined,
  direction: "outbound" | "inbound",
): boolean {
  if (!status) return false;
  return direction === "outbound"
    ? status.outbound === "active"
    : status.inbound === "active";
}
