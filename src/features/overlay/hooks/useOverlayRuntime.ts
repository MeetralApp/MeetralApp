import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";
import { saveConfig } from "@/shared/lib/api/configApi";
import {
  getAppSnapshot,
  setInboundAudioMode,
  setInboundOutputMode,
  setInboundVoiceOutput,
  setMicMuted,
  setOutboundAudioMode,
  setOutboundOutputMode,
  setOutboundVoiceOutput,
  setSpeakerMuted,
} from "@/features/pipeline/api/pipelineApi";
import {
  applyRevision,
  configViewFromState,
  type AppSnapshot,
} from "@/features/pipeline/lib/appState";
import {
  isColumnAudioFaultFromUi,
  selectColumnUi,
  type ColumnUiState,
} from "@/features/pipeline/lib/columnUi";
import {
  getAudioPathMode,
  isColumnAwaitingDirectStandby,
  isDirectionActive,
  normalizeAppStatus,
} from "@/features/pipeline/lib/pipelineStatus";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import {
  getOutboundToolbarModeOptions,
  getPipelineModeOptions,
  inboundToolbarModeFromConfig,
  inboundToolbarPatch,
  outboundToolbarModeFromConfig,
  outboundToolbarPatch,
  type OutboundToolbarMode,
} from "@/features/pipeline/lib/pipelineLabels";
import {
  DEFAULT_OVERLAY_SETTINGS,
  type AppStatus,
  type AudioPathMode,
  type ConfigView,
} from "@/shared/lib/types/pipeline";
import type {
  OverlayColumnRuntime,
  OverlayRuntime,
} from "../lib/overlayRuntimeTypes";

const VOICE_SWITCH_DEBOUNCE_MS = 800;

function emptyColumnRuntime(): OverlayColumnRuntime {
  return {
    column: null,
    pathMode: "direct",
    muteEnabled: false,
    canDirect: false,
    canTranslate: false,
    audioFault: false,
    awaitingDirectStandby: false,
    translateDisabled: true,
    directDisabled: true,
    idleKind: null,
    idleLabel: null,
    idleTitle: null,
  };
}

function deriveColumn(
  snapshot: AppSnapshot | null,
  config: ConfigView | null,
  status: AppStatus | null,
  direction: "outbound" | "inbound",
): OverlayColumnRuntime {
  const column = selectColumnUi(snapshot, direction);
  if (!column || !config) return emptyColumnRuntime();

  // Prefer live runtime status so optimistic starting/stopping paints immediately.
  const pipeline =
    (direction === "outbound" ? status?.outbound : status?.inbound) ??
    column.pipeline;
  const pathMode = getAudioPathMode(status, direction);
  const audioFault = isColumnAudioFaultFromUi(column);
  const awaitingDirectStandby = isColumnAwaitingDirectStandby(
    config,
    status,
    direction,
    column.canDirect,
  );
  const translateDisabled =
    pipeline === "starting" ||
    pipeline === "stopping" ||
    audioFault ||
    (!column.canTranslate && pathMode !== "translate");
  const directDisabled =
    !column.canDirect ||
    awaitingDirectStandby ||
    pipeline === "starting" ||
    pipeline === "stopping";

  const showIdle =
    pipeline !== "starting" &&
    pipeline !== "stopping" &&
    pipeline !== "active" &&
    column.idleBadge?.label;

  return {
    column,
    pathMode,
    muteEnabled: column.muteEnabled,
    canDirect: column.canDirect,
    canTranslate: column.canTranslate,
    audioFault,
    awaitingDirectStandby,
    translateDisabled,
    directDisabled,
    idleKind: showIdle ? column.idleBadge.kind : null,
    idleLabel: showIdle ? column.idleBadge.label : null,
    idleTitle: showIdle ? column.idleBadge.title : null,
  };
}

function isOutboundToolbarMode(value: string): value is OutboundToolbarMode {
  return (
    value === "translated" ||
    value === "translatedCustom" ||
    value === "originalAudio" ||
    value === "textOnly"
  );
}

export function useOverlayRuntime(): OverlayRuntime {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const lastVoiceSwitchAt = useRef(0);

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      try {
        const initial = await getAppSnapshot();
        if (!cancelled) setSnapshot(initial);
      } catch {
      /* ignore bootstrap failure — wait for event */
      }
    })();

    const unlisten = listenSafe<AppSnapshot>(APP_EVENTS.appState, (event) => {
      const next = event.payload;
      if (!next) return;
      setSnapshot((prev) => (prev ? applyRevision(prev, next) : next));
    });

    return () => {
      cancelled = true;
      unlisten();
    };
  }, []);

  const status = useMemo(
    () => (snapshot ? normalizeAppStatus(snapshot.runtime) : null),
    [snapshot],
  );
  const config = useMemo(
    () => (snapshot ? configViewFromState(snapshot.config) : null),
    [snapshot],
  );
  const overlay = useMemo(
    () => ({
      ...DEFAULT_OVERLAY_SETTINGS,
      ...(config?.overlay ?? {}),
    }),
    [config],
  );
  const transcriptLayout = config?.transcriptLayout ?? "sideBySide";
  const outbound = useMemo(
    () => deriveColumn(snapshot, config, status, "outbound"),
    [snapshot, config, status],
  );
  const inbound = useMemo(
    () => deriveColumn(snapshot, config, status, "inbound"),
    [snapshot, config, status],
  );

  const outboundToolbarMode = config
    ? outboundToolbarModeFromConfig(config)
    : "translated";
  const inboundMode = config
    ? inboundToolbarModeFromConfig(config)
    : "translated";
  const outboundModeOptions = useMemo(
    () => (config ? getOutboundToolbarModeOptions(config) : []),
    [config],
  );
  const inboundModeOptions = useMemo(
    () => getPipelineModeOptions("inbound", config ?? undefined),
    [config],
  );
  const customActive =
    outbound.pathMode === "translate" &&
    outboundToolbarMode === "translatedCustom";

  const toggleMicMuted = useCallback(async () => {
    await setMicMuted(!(status?.micMuted ?? false));
  }, [status?.micMuted]);

  const toggleSpeakerMuted = useCallback(async () => {
    await setSpeakerMuted(!(status?.speakerMuted ?? false));
  }, [status?.speakerMuted]);

  const setOutboundPath = useCallback(async (mode: AudioPathMode) => {
    setSnapshot((prev) => {
      if (!prev) return prev;
      if (mode === "translate") {
        return {
          ...prev,
          runtime: { ...prev.runtime, outbound: "starting", error: null },
        };
      }
      if (
        mode === "direct" &&
        (prev.runtime.outbound === "active" ||
          prev.runtime.outbound === "starting")
      ) {
        return {
          ...prev,
          runtime: { ...prev.runtime, outbound: "stopping", error: null },
        };
      }
      return prev;
    });
    try {
      await setOutboundAudioMode(mode);
    } catch (error) {
      try {
        const latest = await getAppSnapshot();
        setSnapshot((prev) => (prev ? applyRevision(prev, latest) : latest));
      } catch {
      /* ignore refresh failure */
      }
      throw error;
    }
  }, []);

  const setInboundPath = useCallback(async (mode: AudioPathMode) => {
    setSnapshot((prev) => {
      if (!prev) return prev;
      if (mode === "translate") {
        return {
          ...prev,
          runtime: { ...prev.runtime, inbound: "starting", error: null },
        };
      }
      if (
        mode === "direct" &&
        (prev.runtime.inbound === "active" ||
          prev.runtime.inbound === "starting")
      ) {
        return {
          ...prev,
          runtime: { ...prev.runtime, inbound: "stopping", error: null },
        };
      }
      return prev;
    });
    try {
      await setInboundAudioMode(mode);
    } catch (error) {
      try {
        const latest = await getAppSnapshot();
        setSnapshot((prev) => (prev ? applyRevision(prev, latest) : latest));
      } catch {
      /* ignore refresh failure */
      }
      throw error;
    }
  }, []);

  const setOutputMode = useCallback(
    async (direction: "outbound" | "inbound", toolbarValue: string) => {
      if (!config) return;

      if (direction === "outbound") {
        if (!isOutboundToolbarMode(toolbarValue)) return;
        const patch = outboundToolbarPatch(toolbarValue, config);
        const next = { ...config, ...patch };
        const active = isDirectionActive(status, "outbound");
        const voiceOutputChanged =
          next.outboundVoiceOutput !== config.outboundVoiceOutput;
        const modeChanged = next.outboundMode !== config.outboundMode;

        if (active) {
          if (modeChanged) {
            await setOutboundOutputMode(next.outboundMode);
          }
          if (
            voiceOutputChanged &&
            next.outboundMode === "translated" &&
            outbound.pathMode === "translate"
          ) {
            const now = Date.now();
            if (now - lastVoiceSwitchAt.current < VOICE_SWITCH_DEBOUNCE_MS) {
              return;
            }
            lastVoiceSwitchAt.current = now;
            await setOutboundVoiceOutput(next.outboundVoiceOutput);
          }
          return;
        }

        await saveConfig(toSavePayload(next));
        return;
      }

      if (!isOutboundToolbarMode(toolbarValue)) return;
      const patch = inboundToolbarPatch(toolbarValue, config);
      const next = { ...config, ...patch };
      const active = isDirectionActive(status, "inbound");
      const voiceOutputChanged =
        next.inboundVoiceOutput !== config.inboundVoiceOutput;
      const modeChanged = next.inboundMode !== config.inboundMode;

      if (active) {
        if (modeChanged) {
          await setInboundOutputMode(next.inboundMode);
        }
        if (
          voiceOutputChanged &&
          next.inboundMode === "translated" &&
          inbound.pathMode === "translate"
        ) {
          const now = Date.now();
          if (now - lastVoiceSwitchAt.current < VOICE_SWITCH_DEBOUNCE_MS) {
            return;
          }
          lastVoiceSwitchAt.current = now;
          await setInboundVoiceOutput(
            next.inboundVoiceOutput ?? "providerNative",
          );
        }
        return;
      }
      await saveConfig(toSavePayload(next));
    },
    [config, inbound.pathMode, outbound.pathMode, status],
  );

  return useMemo(
    () => ({
      status,
      config,
      overlay,
      transcriptLayout,
      outbound,
      inbound,
      micMuted: status?.micMuted ?? false,
      speakerMuted: status?.speakerMuted ?? false,
      outboundToolbarMode,
      inboundMode,
      outboundModeOptions,
      inboundModeOptions,
      customActive,
      toggleMicMuted,
      toggleSpeakerMuted,
      setOutboundPath,
      setInboundPath,
      setOutputMode,
    }),
    [
      status,
      config,
      overlay,
      transcriptLayout,
      outbound,
      inbound,
      outboundToolbarMode,
      inboundMode,
      outboundModeOptions,
      inboundModeOptions,
      customActive,
      toggleMicMuted,
      toggleSpeakerMuted,
      setOutboundPath,
      setInboundPath,
      setOutputMode,
    ],
  );
}

/** Pure gating helper for unit tests. */
export function overlayTranslateDisabled(input: {
  pipeline: ColumnUiState["pipeline"];
  canTranslate: boolean;
  audioFault: boolean;
  pathMode: AudioPathMode;
}): boolean {
  return (
    input.pipeline === "starting" ||
    input.pipeline === "stopping" ||
    input.audioFault ||
    (!input.canTranslate && input.pathMode !== "translate")
  );
}
