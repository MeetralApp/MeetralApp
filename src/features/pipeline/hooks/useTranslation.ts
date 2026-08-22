import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";
import {
  resolveGlobalAppErrorNotice,
  resolvePipelineErrorNotice,
  type PipelineErrorNotice,
} from "@/features/audio/lib/audioSetup";
import {
  applyRevision,
  configViewFromState,
  type AppSnapshot,
  type DeviceCatalogState,
  type SetupState,
} from "../lib/appState";
import { selectColumnUi } from "../lib/columnUi";
import type {
  AppStatus,
  AudioPathMode,
  ConfigView,
  DevicesResponse,
  ElevenLabsModelOption,
  ElevenLabsVoiceOption,
  InboundVoiceOutput,
  OutboundVoiceOutput,
  PipelineOutputMode,
  SaveConfigPayload,
  SonioxVoiceOption,
} from "@/shared/lib/types/pipeline";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import { normalizeAppStatus } from "../lib/pipelineStatus";
import * as pipelineApi from "@/features/pipeline/api/pipelineApi";
import * as configApi from "@/shared/lib/api/configApi";
import * as voiceApi from "@/shared/lib/api/voiceApi";

function applySnapshot(snapshot: AppSnapshot) {
  return {
    status: normalizeAppStatus(snapshot.runtime),
    setup: snapshot.setup,
    devices: snapshot.devices,
    config: configViewFromState(snapshot.config),
    appSnapshot: snapshot,
  };
}

export function useTranslation() {
  const [config, setConfig] = useState<ConfigView | null>(null);
  const [deviceCatalog, setDeviceCatalog] =
    useState<DeviceCatalogState | null>(null);
  const [setup, setSetup] = useState<SetupState | null>(null);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [appSnapshot, setAppSnapshot] = useState<AppSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<PipelineErrorNotice | null>(null);
  const snapshotRef = useRef<AppSnapshot | null>(null);

  const refreshDeviceCatalog = useCallback(async () => {
    const catalog = await pipelineApi.refreshDeviceCatalog();
    setDeviceCatalog((previous) => applyRevision(previous, catalog));
    return catalog;
  }, []);

  const refreshStatus = useCallback(async () => {
    const data = await pipelineApi.getStatus();
    setStatus(data);
    return data;
  }, []);

  const bootstrapAppState = useCallback(async () => {
    const snapshot = await pipelineApi.getAppSnapshot();
    const applied = applySnapshot(snapshot);
    snapshotRef.current = applied.appSnapshot;
    setStatus(applied.status);
    setSetup(applied.setup);
    setDeviceCatalog(applied.devices);
    setConfig(applied.config);
    setAppSnapshot(applied.appSnapshot);
    setError(resolveGlobalAppErrorNotice(applied.status));
    return snapshot;
  }, []);

  const saveConfig = useCallback(async (next: SaveConfigPayload) => {
    return configApi.saveConfig(next);
  }, []);

  const setOutboundOutputMode = useCallback(
    async (mode: PipelineOutputMode) => {
      setError(null);
      try {
        const nextStatus = await pipelineApi.setOutboundOutputMode(mode);
        setStatus(nextStatus);
        if (nextStatus.error) {
          setError(resolvePipelineErrorNotice(nextStatus.error));
        }
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const setOutboundVoiceOutput = useCallback(
    async (voiceOutput: OutboundVoiceOutput) => {
      setError(null);
      try {
        const nextStatus = await pipelineApi.setOutboundVoiceOutput(voiceOutput);
        setStatus(nextStatus);
        if (nextStatus.error) {
          setError(resolvePipelineErrorNotice(nextStatus.error));
        }
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const setInboundOutputMode = useCallback(
    async (mode: PipelineOutputMode) => {
      setError(null);
      try {
        const nextStatus = await pipelineApi.setInboundOutputMode(mode);
        setStatus(nextStatus);
        if (nextStatus.error) {
          setError(resolvePipelineErrorNotice(nextStatus.error));
        }
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const setInboundVoiceOutput = useCallback(
    async (voiceOutput: InboundVoiceOutput) => {
      setError(null);
      try {
        const nextStatus = await pipelineApi.setInboundVoiceOutput(voiceOutput);
        setStatus(nextStatus);
        if (nextStatus.error) {
          setError(resolvePipelineErrorNotice(nextStatus.error));
        }
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const setOutboundAudioMode = useCallback(
    async (mode: AudioPathMode) => {
      setError(null);
      if (mode === "translate") {
        setStatus((prev) =>
          prev ? { ...prev, outbound: "starting", error: null } : prev,
        );
      } else if (mode === "direct") {
        setStatus((prev) =>
          prev && (prev.outbound === "active" || prev.outbound === "starting")
            ? { ...prev, outbound: "stopping", error: null }
            : prev,
        );
      }
      try {
        const nextStatus = await pipelineApi.setOutboundAudioMode(mode);
        setStatus(nextStatus);
        setError(resolveGlobalAppErrorNotice(nextStatus));
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const setInboundAudioMode = useCallback(
    async (mode: AudioPathMode) => {
      setError(null);
      if (mode === "translate") {
        setStatus((prev) =>
          prev ? { ...prev, inbound: "starting", error: null } : prev,
        );
      } else if (mode === "direct") {
        setStatus((prev) =>
          prev && (prev.inbound === "active" || prev.inbound === "starting")
            ? { ...prev, inbound: "stopping", error: null }
            : prev,
        );
      }
      try {
        const nextStatus = await pipelineApi.setInboundAudioMode(mode);
        setStatus(nextStatus);
        setError(resolveGlobalAppErrorNotice(nextStatus));
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const ensureDirectAudio = useCallback(async () => {
    try {
      const nextStatus = await pipelineApi.ensureDirectAudio();
      setStatus(nextStatus);
    } catch (e) {
      setError(resolvePipelineErrorNotice(String(e)));
    }
  }, []);

  const setMicMuted = useCallback(
    async (muted: boolean) => {
      setError(null);
      try {
        const nextStatus = await pipelineApi.setMicMuted(muted);
        setStatus(nextStatus);
        if (nextStatus.error) {
          setError(resolvePipelineErrorNotice(nextStatus.error));
        }
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const setSpeakerMuted = useCallback(
    async (muted: boolean) => {
      setError(null);
      try {
        const nextStatus = await pipelineApi.setSpeakerMuted(muted);
        setStatus(nextStatus);
        if (nextStatus.error) {
          setError(resolvePipelineErrorNotice(nextStatus.error));
        }
      } catch (e) {
        setError(resolvePipelineErrorNotice(String(e)));
        await refreshStatus();
        throw e;
      }
    },
    [refreshStatus],
  );

  const testApiKey = useCallback(
    async (
      apiKey: string,
      providerOverride?: AiProvider,
      purpose: "live" | "summary" = "live",
    ) => {
      const provider = providerOverride ?? config?.aiProvider ?? "gemini";
      await configApi.testApiKey(apiKey, provider, purpose);
    },
    [config?.aiProvider],
  );

  const testElevenLabsApiKey = useCallback(async (apiKey: string) => {
    await voiceApi.testElevenLabsApiKey(apiKey);
  }, []);

  const listElevenLabsVoices = useCallback(
    async (apiKey = ""): Promise<ElevenLabsVoiceOption[]> => {
      return voiceApi.listElevenLabsVoices(apiKey);
    },
    [],
  );

  const listSonioxVoices = useCallback(
    async (apiKey = "", preferredModel?: string): Promise<SonioxVoiceOption[]> => {
      return voiceApi.listSonioxVoices(apiKey, preferredModel);
    },
    [],
  );

  const listElevenLabsModels = useCallback(
    async (apiKey = ""): Promise<ElevenLabsModelOption[]> => {
      return voiceApi.listElevenLabsModels(apiKey);
    },
    [],
  );

  const validateElevenLabsVoice = useCallback(
    async (voiceId: string, apiKey = "") => {
      await voiceApi.validateElevenLabsVoice(voiceId, apiKey);
    },
    [],
  );

  const previewElevenLabsVoice = useCallback(
    async (voiceId: string, apiKey = "") => {
      await voiceApi.previewElevenLabsVoice(voiceId, apiKey);
    },
    [],
  );

  const previewSonioxVoice = useCallback(
    async (voice: string, apiKey = "") => {
      await voiceApi.previewSonioxVoice(voice, apiKey);
    },
    [],
  );

  useEffect(() => {
    let cancelled = false;

    const unlisten = listenSafe<AppSnapshot>(APP_EVENTS.appState, (event) => {
      if (cancelled) return;
      // Compute outside any setState updater (updaters must be pure — React may
      // double-invoke them). The ref mirrors the snapshot so revisions stay
      // ordered without a functional update. React 18+ auto-batches the setters.
      const next = applyRevision(snapshotRef.current, event.payload);
      snapshotRef.current = next;
      const applied = applySnapshot(next);
      setAppSnapshot(next);
      setStatus(applied.status);
      setSetup(applied.setup);
      setDeviceCatalog(applied.devices);
      setConfig(applied.config);
      setError(resolveGlobalAppErrorNotice(applied.status));
    });

    void (async () => {
      try {
        await bootstrapAppState();
      } catch (e) {
        if (!cancelled) {
          setError(resolvePipelineErrorNotice(String(e)));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
      unlisten();
    };
  }, [bootstrapAppState]);

  const devices = useMemo<DevicesResponse | null>(
    () => (deviceCatalog ? { devices: deviceCatalog.devices } : null),
    [deviceCatalog],
  );

  const columnUi = useCallback(
    (direction: "outbound" | "inbound") => selectColumnUi(appSnapshot, direction),
    [appSnapshot],
  );

  return useMemo(
    () => ({
      config,
      devices,
      setup,
      status,
      appSnapshot,
      loading,
      error,
      setError,
      saveConfig,
      refreshDeviceCatalog,
      setOutboundAudioMode,
      setInboundAudioMode,
      ensureDirectAudio,
      setOutboundOutputMode,
      setOutboundVoiceOutput,
      setInboundOutputMode,
      setInboundVoiceOutput,
      setMicMuted,
      setSpeakerMuted,
      testApiKey,
      testElevenLabsApiKey,
      listElevenLabsVoices,
      listSonioxVoices,
      listElevenLabsModels,
      validateElevenLabsVoice,
      previewElevenLabsVoice,
      previewSonioxVoice,
      columnUi,
    }),
    [
      config,
      devices,
      setup,
      status,
      appSnapshot,
      loading,
      error,
      saveConfig,
      refreshDeviceCatalog,
      setOutboundAudioMode,
      setInboundAudioMode,
      ensureDirectAudio,
      setOutboundOutputMode,
      setOutboundVoiceOutput,
      setInboundOutputMode,
      setInboundVoiceOutput,
      setMicMuted,
      setSpeakerMuted,
      testApiKey,
      testElevenLabsApiKey,
      listElevenLabsVoices,
      listSonioxVoices,
      listElevenLabsModels,
      validateElevenLabsVoice,
      previewElevenLabsVoice,
      previewSonioxVoice,
      columnUi,
    ],
  );
}
