import { useRef } from "react";
import TranscriptSplit from "./TranscriptSplit";
import type { ToastType } from "@/shared/context/toastTypes";
import {
  inboundToolbarPatch,
  outboundToolbarPatch,
  type InboundToolbarMode,
  type OutboundToolbarMode,
} from "../lib/pipelineLabels";
import type { AppStatus, AudioPathMode, ConfigView, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { isDirectionActive } from "../lib/pipelineStatus";
import type { ColumnUiState } from "../lib/columnUi";
import { toSavePayload } from "../lib/toSavePayload";

const VOICE_SWITCH_DEBOUNCE_MS = 800;

interface Props {
  config: ConfigView;
  status: AppStatus | null;
  outboundColumn: ColumnUiState;
  inboundColumn: ColumnUiState;
  onSaveMode: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onSetOutboundOutputMode: (mode: PipelineOutputMode) => Promise<void>;
  onSetOutboundVoiceOutput: (voiceOutput: OutboundVoiceOutput) => Promise<void>;
  onSetInboundOutputMode: (mode: PipelineOutputMode) => Promise<void>;
  onSetInboundVoiceOutput: (voiceOutput: InboundVoiceOutput) => Promise<void>;
  onOutboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onInboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onLongReconnectRestored?: (columnTitle: string) => void;
  onLongAudioReconnectRestored?: (columnTitle: string) => void;
  onOpenSettings?: () => void;
  suppressSetupIdleChip?: boolean;
  onToast?: (type: ToastType, text: string) => void;
  micMuted?: boolean;
  onMicMuteToggle?: () => void;
  speakerMuted?: boolean;
  onSpeakerMuteToggle?: () => void;
}

export default function TranscriptPanel({
  config,
  status,
  outboundColumn,
  inboundColumn,
  onSaveMode,
  onSetOutboundOutputMode,
  onSetOutboundVoiceOutput,
  onSetInboundOutputMode,
  onSetInboundVoiceOutput,
  onOutboundAudioPathChange,
  onInboundAudioPathChange,
  onLongReconnectRestored,
  onLongAudioReconnectRestored,
  onOpenSettings,
  suppressSetupIdleChip = false,
  onToast,
  micMuted,
  onMicMuteToggle,
  speakerMuted,
  onSpeakerMuteToggle,
}: Props) {
  const lastVoiceSwitchAt = useRef(0);

  const isOutboundToolbarMode = (value: string): value is OutboundToolbarMode =>
    value === "translated" ||
    value === "translatedClone" ||
    value === "originalAudio" ||
    value === "textOnly";

  const saveOutboundToolbar = async (toolbarMode: OutboundToolbarMode) => {
    const patch = outboundToolbarPatch(toolbarMode, config);
    const next = { ...config, ...patch };
    const active = isDirectionActive(status, "outbound");
    const voiceOutputChanged =
      next.outboundVoiceOutput !== config.outboundVoiceOutput;
    const modeChanged = next.outboundMode !== config.outboundMode;

    if (active) {
      if (modeChanged) {
        await onSetOutboundOutputMode(next.outboundMode);
      }
      if (
        voiceOutputChanged &&
        next.outboundMode === "translated" &&
        pathModeIsTranslate(status, "outbound")
      ) {
        const now = Date.now();
        if (now - lastVoiceSwitchAt.current < VOICE_SWITCH_DEBOUNCE_MS) {
          return;
        }
        lastVoiceSwitchAt.current = now;
        try {
          await onSetOutboundVoiceOutput(next.outboundVoiceOutput);
        } catch (e) {
          onToast?.("error", String(e));
        }
      }
      return;
    }

    await onSaveMode(toSavePayload(next));
  };

  const isInboundToolbarMode = (value: string): value is InboundToolbarMode =>
    value === "translated" ||
    value === "translatedClone" ||
    value === "originalAudio" ||
    value === "textOnly";

  const saveInboundToolbar = async (toolbarMode: InboundToolbarMode) => {
    const patch = inboundToolbarPatch(toolbarMode, config);
    const next = { ...config, ...patch };
    const active = isDirectionActive(status, "inbound");
    const voiceOutputChanged =
      next.inboundVoiceOutput !== config.inboundVoiceOutput;
    const modeChanged = next.inboundMode !== config.inboundMode;

    if (active) {
      if (modeChanged) {
        await onSetInboundOutputMode(next.inboundMode);
      }
      if (
        voiceOutputChanged &&
        next.inboundMode === "translated" &&
        pathModeIsTranslate(status, "inbound")
      ) {
        const now = Date.now();
        if (now - lastVoiceSwitchAt.current < VOICE_SWITCH_DEBOUNCE_MS) {
          return;
        }
        lastVoiceSwitchAt.current = now;
        try {
          await onSetInboundVoiceOutput(
            next.inboundVoiceOutput ?? "providerNative",
          );
        } catch (e) {
          onToast?.("error", String(e));
        }
      }
      return;
    }

    await onSaveMode(toSavePayload(next));
  };

  const saveMode = async (
    direction: "outbound" | "inbound",
    mode: PipelineOutputMode,
  ) => {
    const active = isDirectionActive(status, direction);

    if (active) {
      if (direction === "outbound") {
        await onSetOutboundOutputMode(mode);
      } else {
        await onSetInboundOutputMode(mode);
      }
      return;
    }

    const next: ConfigView =
      direction === "outbound"
        ? { ...config, outboundMode: mode }
        : { ...config, inboundMode: mode };
    await onSaveMode(toSavePayload(next));
  };

  function pathModeIsTranslate(
    appStatus: AppStatus | null | undefined,
    direction: "outbound" | "inbound",
  ): boolean {
    if (!appStatus) return false;
    const pipeline = appStatus[direction];
    return pipeline === "active" || pipeline === "starting" || pipeline === "stopping";
  }

  return (
    <TranscriptSplit
      config={config}
      status={status}
      outboundColumn={outboundColumn}
      inboundColumn={inboundColumn}
      onOutboundAudioPathChange={onOutboundAudioPathChange}
      onInboundAudioPathChange={onInboundAudioPathChange}
      onOutboundModeChange={(mode) => {
        if (isOutboundToolbarMode(mode)) {
          return saveOutboundToolbar(mode);
        }
        return saveMode("outbound", mode as PipelineOutputMode);
      }}
      onInboundModeChange={(mode) => {
        if (isInboundToolbarMode(mode)) {
          return saveInboundToolbar(mode);
        }
        return saveMode("inbound", mode as PipelineOutputMode);
      }}
      onLongReconnectRestored={onLongReconnectRestored}
      onLongAudioReconnectRestored={onLongAudioReconnectRestored}
      onOpenSettings={onOpenSettings}
      suppressSetupIdleChip={suppressSetupIdleChip}
      micMuted={micMuted}
      onMicMuteToggle={onMicMuteToggle}
      speakerMuted={speakerMuted}
      onSpeakerMuteToggle={onSpeakerMuteToggle}
    />
  );
}
