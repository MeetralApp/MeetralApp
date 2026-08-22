import type { AppStatus, PipelineState } from "@/shared/lib/types/pipeline";
import type { AudioConnectionState } from "@/features/audio/lib/audioConnection";
import type { PipelineDirection } from "./sessionDrift";
import type { AppSnapshot, ColumnIdleBadgeSnapshot, SetupState } from "./appState";

export interface ColumnUiState {
  pipeline: PipelineState;
  audioConnection: AudioConnectionState;
  canDirect: boolean;
  canTranslate: boolean;
  translateDisabledReason: string | null;
  idleBadge: ColumnIdleBadgeSnapshot;
  pipelineLive: boolean;
  muteEnabled: boolean;
}

function normalizePipelineState(state: string): PipelineState {
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

function normalizeAudioConnection(
  value: string | undefined,
): AudioConnectionState {
  if (value === "reconnecting" || value === "lost") return value;
  return "ok";
}

function normalizeColumnUi(raw: ColumnUiState): ColumnUiState {
  return {
    ...raw,
    pipeline: normalizePipelineState(raw.pipeline),
    audioConnection: normalizeAudioConnection(raw.audioConnection),
  };
}

export function selectColumnUi(
  snapshot: AppSnapshot | null,
  direction: PipelineDirection,
): ColumnUiState | null {
  if (!snapshot?.columns) return null;
  const column =
    direction === "outbound"
      ? snapshot.columns.outbound
      : snapshot.columns.inbound;
  return normalizeColumnUi(column);
}

/** Fallback when only setup + status available (legacy paths). */
export function columnUiFromLegacy(
  setup: SetupState,
  status: AppStatus | null,
  direction: PipelineDirection,
): ColumnUiState {
  const pipeline =
    direction === "outbound"
      ? (status?.outbound ?? "off")
      : (status?.inbound ?? "off");
  const audioConnection =
    direction === "outbound"
      ? (status?.outboundAudio ?? "ok")
      : (status?.inboundAudio ?? "ok");
  const canDirect =
    direction === "outbound"
      ? setup.canDirectOutbound
      : setup.canDirectInbound;
  const canTranslate =
    direction === "outbound"
      ? setup.canTranslateOutbound
      : setup.canTranslateInbound;
  const idleBadge =
    direction === "outbound"
      ? setup.outboundIdleBadge
      : setup.inboundIdleBadge;
  const pipelineLive =
    pipeline === "direct" ||
    pipeline === "starting" ||
    pipeline === "stopping" ||
    pipeline === "active";

  return {
    pipeline,
    audioConnection,
    canDirect,
    canTranslate,
    translateDisabledReason: canTranslate
      ? null
      : audioConnection === "lost"
        ? "Audio device disconnected"
        : audioConnection === "reconnecting"
          ? "Audio device reconnecting"
          : null,
    idleBadge,
    pipelineLive,
    muteEnabled: pipelineLive,
  };
}

export function isColumnAudioFaultFromUi(column: ColumnUiState): boolean {
  return (
    column.audioConnection === "reconnecting" ||
    column.audioConnection === "lost"
  );
}

export function isTranslateEnabled(column: ColumnUiState): boolean {
  return column.canTranslate && !isColumnAudioFaultFromUi(column);
}
