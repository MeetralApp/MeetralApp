import { useCallback, useEffect, useRef, useState } from "react";

import SettingsSection from "@/features/config/components/SettingsSection";
import type { SettingsSectionStatus } from "@/features/config/lib/settingsDrawerStatus";
import type { SettingsFocus } from "@/features/config/lib/settingsFocus";
import type { ToastType } from "@/shared/context/toastTypes";
import {
  engineVoiceHint,
  FALLBACK_SONIOX_VOICES,
  inboundVoiceNote,
} from "@/features/voice/lib/voiceSettings";
import { useVoiceCatalog } from "@/features/voice/hooks/useVoiceCatalog";
import { listSonioxTtsModels } from "@/features/ai/lib/aiApi";
import type { SonioxTtsModelOption } from "@/features/ai/lib/aiTypes";
import { withLanguageFlags } from "@/shared/lib/languageFlags";
import type { CustomVoiceVendor, ConfigView, ElevenLabsModelOption, ElevenLabsVoiceOption, FishAudioLatency, FishAudioModelOption, FishAudioVoiceOption, InboundVoiceOutput, OutboundVoiceOutput, SonioxVoiceOption, TtsSynthesisMode, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { isCustomVoiceOutput, normalizeCustomVoiceVendor, normalizeVoiceOutput } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";

import CustomVoiceVendorSelect from "./CustomVoiceVendorSelect";
import ElevenLabsCustomVoicePanel from "./ElevenLabsCustomVoicePanel";
import FishAudioCustomVoicePanel from "./FishAudioCustomVoicePanel";
import InboundVoiceModeSelect from "./InboundVoiceModeSelect";
import OutboundVoiceModeSelect from "./OutboundVoiceModeSelect";
import SonioxEngineVoiceSettings from "./SonioxEngineVoiceSettings";
import SonioxInboundVoiceSettings from "./SonioxInboundVoiceSettings";

export interface VoiceSettingsState {
  dirty: boolean;
}

type VoiceSession = {
  inboundOutputMode: InboundVoiceOutput;
  outputMode: OutboundVoiceOutput;
  inboundCustomVoiceVendor: CustomVoiceVendor;
  outboundCustomVoiceVendor: CustomVoiceVendor;
  sonioxTtsModel: string;
};

type VoiceSavePatch = Parameters<typeof toSavePayload>[1];

interface Props {
  config: ConfigView;
  outboundLocked: boolean;
  inboundLocked: boolean;
  status?: SettingsSectionStatus;
  focusKey?: SettingsFocus;
  activeFocus?: SettingsFocus | null;
  /** Deep-link into custom voice / vendor block when Settings opens with focus `customVoice`. */
  focusTarget?: "customVoice";
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
  onStateChange?: (state: VoiceSettingsState) => void;
}

export default function VoiceSettings({
  config,
  outboundLocked,
  inboundLocked,
  status,
  focusKey,
  activeFocus,
  focusTarget,
  onSave,
  onToast,
  onStateChange,
}: Props) {
  const {
    listElevenLabsVoices,
    listElevenLabsModels,
    listSonioxVoices,
    validateElevenLabsVoice,
    previewElevenLabsVoice,
    previewSonioxVoice,
    testElevenLabsApiKey,
    testFishAudioApiKey,
    listFishAudioVoices,
    listFishAudioModels,
    validateFishAudioVoice,
    previewFishAudioVoice,
  } = useVoiceCatalog();
  const [inboundOutputMode, setInboundOutputMode] =
    useState<InboundVoiceOutput>(
      normalizeVoiceOutput(config.inboundVoiceOutput),
    );
  const [outputMode, setOutputMode] = useState<OutboundVoiceOutput>(
    normalizeVoiceOutput(config.outboundVoiceOutput),
  );
  const [inboundCustomVoiceVendor, setInboundCustomVoiceVendor] = useState<CustomVoiceVendor>(
    normalizeCustomVoiceVendor(config.inboundCustomVoiceVendor),
  );
  const [outboundCustomVoiceVendor, setOutboundCustomVoiceVendor] = useState<CustomVoiceVendor>(
    normalizeCustomVoiceVendor(config.outboundCustomVoiceVendor),
  );
  const [inboundVendorSaving, setInboundVendorSaving] = useState(false);
  const [outboundVendorSaving, setOutboundVendorSaving] = useState(false);
  const [keyDirty, setKeyDirty] = useState(false);
  const [voiceDirty, setVoiceDirty] = useState(false);
  const [inboundModeSaving, setInboundModeSaving] = useState(false);
  const [modeSaving, setModeSaving] = useState(false);
  const [voicesNonce, setVoicesNonce] = useState(0);
  const [sonioxVoices, setSonioxVoices] = useState<SonioxVoiceOption[]>(() => {
    const cached = config.sonioxTtsVoices ?? [];
    return cached.length > 0 ? cached : FALLBACK_SONIOX_VOICES;
  });
  const [sonioxVoicesLoading, setSonioxVoicesLoading] = useState(false);
  const [sonioxTtsModels, setSonioxTtsModels] = useState<SonioxTtsModelOption[]>(
    () =>
      (config.sonioxTtsModels ?? []).map((model) => ({
        ...model,
        languages: withLanguageFlags(model.languages ?? []),
      })),
  );
  const [ttsModel, setTtsModel] = useState(
    config.elevenlabsTtsModel ?? "eleven_flash_v2_5",
  );
  const [stability, setStability] = useState(
    config.elevenlabsStability ?? 0.5,
  );
  const [similarityBoost, setSimilarityBoost] = useState(
    config.elevenlabsSimilarityBoost ?? 0.75,
  );
  const [synthesisMode, setSynthesisMode] = useState<TtsSynthesisMode>(
    config.elevenlabsTtsSynthesisMode ?? "streaming",
  );
  const [inboundTtsModel, setInboundTtsModel] = useState(
    config.elevenlabsInboundTtsModel ?? "eleven_flash_v2_5",
  );
  const [inboundStability, setInboundStability] = useState(
    config.elevenlabsInboundStability ?? 0.5,
  );
  const [inboundSimilarityBoost, setInboundSimilarityBoost] = useState(
    config.elevenlabsInboundSimilarityBoost ?? 0.75,
  );
  const [inboundSynthesisMode, setInboundSynthesisMode] =
    useState<TtsSynthesisMode>(
      config.elevenlabsInboundTtsSynthesisMode ?? "streaming",
    );
  const [inboundCustomVoiceSettingsSaving, setInboundCustomVoiceSettingsSaving] =
    useState(false);
  const [customVoiceSettingsSaving, setCustomVoiceSettingsSaving] = useState(false);
  const [fishTtsModel, setFishTtsModel] = useState(
    config.fishaudioTtsModel ?? "s2.1-pro",
  );
  const [fishLatency, setFishLatency] = useState<FishAudioLatency>(
    config.fishaudioLatency ?? "balanced",
  );
  const [fishTemperature, setFishTemperature] = useState(
    config.fishaudioTemperature ?? 0.7,
  );
  const [fishSpeed, setFishSpeed] = useState(config.fishaudioSpeed ?? 1.0);
  const [fishTopP, setFishTopP] = useState(config.fishaudioTopP ?? 0.7);
  const [inboundFishTtsModel, setInboundFishTtsModel] = useState(
    config.fishaudioInboundTtsModel ?? "s2.1-pro",
  );
  const [inboundFishLatency, setInboundFishLatency] = useState<FishAudioLatency>(
    config.fishaudioInboundLatency ?? "balanced",
  );
  const [inboundFishTemperature, setInboundFishTemperature] = useState(
    config.fishaudioInboundTemperature ?? 0.7,
  );
  const inboundCustomVoiceSectionRef = useRef<HTMLDivElement>(null);
  const outboundCustomVoiceSectionRef = useRef<HTMLDivElement>(null);
  const configRef = useRef(config);
  configRef.current = config;
  const sessionRef = useRef<VoiceSession>({
    inboundOutputMode,
    outputMode,
    inboundCustomVoiceVendor,
    outboundCustomVoiceVendor,
    sonioxTtsModel: config.sonioxTtsModel ?? "tts-rt-v1",
  });
  sessionRef.current.inboundOutputMode = inboundOutputMode;
  sessionRef.current.outputMode = outputMode;
  sessionRef.current.inboundCustomVoiceVendor = inboundCustomVoiceVendor;
  sessionRef.current.outboundCustomVoiceVendor = outboundCustomVoiceVendor;

  const speakingStyleAvailable = config.aiProvider !== "soniox";
  const customVoiceSettingsDirty =
    ttsModel !== (config.elevenlabsTtsModel ?? "eleven_flash_v2_5") ||
    stability !== (config.elevenlabsStability ?? 0.5) ||
    similarityBoost !== (config.elevenlabsSimilarityBoost ?? 0.75) ||
    (speakingStyleAvailable &&
      synthesisMode !== (config.elevenlabsTtsSynthesisMode ?? "streaming"));

  const inboundCustomVoiceSettingsDirty =
    inboundTtsModel !==
      (config.elevenlabsInboundTtsModel ?? "eleven_flash_v2_5") ||
    inboundStability !== (config.elevenlabsInboundStability ?? 0.5) ||
    inboundSimilarityBoost !==
      (config.elevenlabsInboundSimilarityBoost ?? 0.75) ||
    (speakingStyleAvailable &&
      inboundSynthesisMode !==
        (config.elevenlabsInboundTtsSynthesisMode ?? "streaming"));

  const inboundCustomVoiceEnabled = isCustomVoiceOutput(inboundOutputMode);
  const customVoiceEnabled = isCustomVoiceOutput(outputMode);
  const inboundUsesEl = inboundCustomVoiceEnabled && inboundCustomVoiceVendor === "elevenLabs";
  const outboundUsesEl = customVoiceEnabled && outboundCustomVoiceVendor === "elevenLabs";
  const inboundUsesFish = inboundCustomVoiceEnabled && inboundCustomVoiceVendor === "fishAudio";
  const outboundUsesFish = customVoiceEnabled && outboundCustomVoiceVendor === "fishAudio";
  const fishCustomVoiceSettingsDirty =
    fishTtsModel !== (config.fishaudioTtsModel ?? "s2.1-pro") ||
    fishLatency !== (config.fishaudioLatency ?? "balanced") ||
    fishTemperature !== (config.fishaudioTemperature ?? 0.7) ||
    fishSpeed !== (config.fishaudioSpeed ?? 1.0) ||
    fishTopP !== (config.fishaudioTopP ?? 0.7);
  const inboundFishCustomVoiceSettingsDirty =
    inboundFishTtsModel !== (config.fishaudioInboundTtsModel ?? "s2.1-pro") ||
    inboundFishLatency !== (config.fishaudioInboundLatency ?? "balanced") ||
    inboundFishTemperature !== (config.fishaudioInboundTemperature ?? 0.7) ||
    fishSpeed !== (config.fishaudioSpeed ?? 1.0) ||
    fishTopP !== (config.fishaudioTopP ?? 0.7);
  const dirty =
    keyDirty ||
    voiceDirty ||
    (inboundUsesEl && inboundCustomVoiceSettingsDirty) ||
    (outboundUsesEl && customVoiceSettingsDirty) ||
    (inboundUsesFish && inboundFishCustomVoiceSettingsDirty) ||
    (outboundUsesFish && fishCustomVoiceSettingsDirty);

  useEffect(() => {
    setTtsModel(config.elevenlabsTtsModel ?? "eleven_flash_v2_5");
    setStability(config.elevenlabsStability ?? 0.5);
    setSimilarityBoost(config.elevenlabsSimilarityBoost ?? 0.75);
    setSynthesisMode(config.elevenlabsTtsSynthesisMode ?? "streaming");
  }, [
    config.elevenlabsTtsModel,
    config.elevenlabsStability,
    config.elevenlabsSimilarityBoost,
    config.elevenlabsTtsSynthesisMode,
  ]);

  useEffect(() => {
    setInboundTtsModel(
      config.elevenlabsInboundTtsModel ?? "eleven_flash_v2_5",
    );
    setInboundStability(config.elevenlabsInboundStability ?? 0.5);
    setInboundSimilarityBoost(
      config.elevenlabsInboundSimilarityBoost ?? 0.75,
    );
    setInboundSynthesisMode(
      config.elevenlabsInboundTtsSynthesisMode ?? "streaming",
    );
  }, [
    config.elevenlabsInboundTtsModel,
    config.elevenlabsInboundStability,
    config.elevenlabsInboundSimilarityBoost,
    config.elevenlabsInboundTtsSynthesisMode,
  ]);

  useEffect(() => {
    setInboundOutputMode(normalizeVoiceOutput(config.inboundVoiceOutput));
  }, [config.inboundVoiceOutput]);

  useEffect(() => {
    setOutputMode(normalizeVoiceOutput(config.outboundVoiceOutput));
  }, [config.outboundVoiceOutput]);

  useEffect(() => {
    setInboundCustomVoiceVendor(normalizeCustomVoiceVendor(config.inboundCustomVoiceVendor));
  }, [config.inboundCustomVoiceVendor]);

  useEffect(() => {
    setOutboundCustomVoiceVendor(normalizeCustomVoiceVendor(config.outboundCustomVoiceVendor));
  }, [config.outboundCustomVoiceVendor]);

  useEffect(() => {
    sessionRef.current.sonioxTtsModel = config.sonioxTtsModel ?? "tts-rt-v1";
  }, [config.sonioxTtsModel]);

  useEffect(() => {
    setFishTtsModel(config.fishaudioTtsModel ?? "s2.1-pro");
    setFishLatency(config.fishaudioLatency ?? "balanced");
    setFishTemperature(config.fishaudioTemperature ?? 0.7);
    setFishSpeed(config.fishaudioSpeed ?? 1.0);
    setFishTopP(config.fishaudioTopP ?? 0.7);
  }, [
    config.fishaudioTtsModel,
    config.fishaudioLatency,
    config.fishaudioTemperature,
    config.fishaudioSpeed,
    config.fishaudioTopP,
  ]);

  useEffect(() => {
    setInboundFishTtsModel(config.fishaudioInboundTtsModel ?? "s2.1-pro");
    setInboundFishLatency(config.fishaudioInboundLatency ?? "balanced");
    setInboundFishTemperature(config.fishaudioInboundTemperature ?? 0.7);
  }, [
    config.fishaudioInboundTtsModel,
    config.fishaudioInboundLatency,
    config.fishaudioInboundTemperature,
  ]);

  const buildVoiceSave = useCallback((patch: VoiceSavePatch = {}) => {
    const session = sessionRef.current;
    return toSavePayload(configRef.current, {
      inboundVoiceOutput: session.inboundOutputMode,
      outboundVoiceOutput: session.outputMode,
      inboundCustomVoiceVendor: session.inboundCustomVoiceVendor,
      outboundCustomVoiceVendor: session.outboundCustomVoiceVendor,
      sonioxTtsModel: session.sonioxTtsModel,
      ...patch,
    });
  }, []);

  const saveVoicePatch = useCallback(
    (patch: VoiceSavePatch = {}) => {
      if (patch.sonioxTtsModel) {
        sessionRef.current.sonioxTtsModel = patch.sonioxTtsModel;
      }
      return onSave(buildVoiceSave(patch));
    },
    [buildVoiceSave, onSave],
  );

  const saveKeepingCloneSession = useCallback(
    (payload: SaveConfigPayload) => {
      const session = sessionRef.current;
      const notesMode =
        (configRef.current.sessionMode ?? "interpreter") === "notes";
      return onSave({
        ...payload,
        inboundVoiceOutput: session.inboundOutputMode,
        outboundVoiceOutput: session.outputMode,
        inboundCustomVoiceVendor: session.inboundCustomVoiceVendor,
        outboundCustomVoiceVendor: session.outboundCustomVoiceVendor,
        sonioxTtsModel:
          payload.sonioxTtsModel === undefined
            ? undefined
            : session.sonioxTtsModel,
        ...(notesMode
          ? {}
          : {
              interpreterInboundVoiceOutput: session.inboundOutputMode,
              interpreterOutboundVoiceOutput: session.outputMode,
            }),
      });
    },
    [onSave],
  );

  const persistElevenLabsVoices = useCallback(
    async (list: ElevenLabsVoiceOption[]) => {
      await onSave(buildVoiceSave({ elevenlabsVoices: list }));
    },
    [buildVoiceSave, onSave],
  );

  const persistElevenLabsModels = useCallback(
    async (list: ElevenLabsModelOption[]) => {
      await onSave(buildVoiceSave({ elevenlabsModels: list }));
    },
    [buildVoiceSave, onSave],
  );

  const persistFishAudioVoices = useCallback(
    async (list: FishAudioVoiceOption[]) => {
      await onSave(buildVoiceSave({ fishaudioVoices: list }));
    },
    [buildVoiceSave, onSave],
  );

  const persistFishAudioModels = useCallback(
    async (list: FishAudioModelOption[]) => {
      await onSave(buildVoiceSave({ fishaudioModels: list }));
    },
    [buildVoiceSave, onSave],
  );

  const refreshSonioxVoices = useCallback(
    async (opts?: { silent?: boolean }) => {
      const snapshot = configRef.current;
      if (!snapshot.sonioxApiKeyConfigured) {
        setSonioxVoices(
          (snapshot.sonioxTtsVoices?.length ?? 0) > 0
            ? (snapshot.sonioxTtsVoices ?? FALLBACK_SONIOX_VOICES)
            : FALLBACK_SONIOX_VOICES,
        );
        setSonioxTtsModels(
          (snapshot.sonioxTtsModels ?? []).map((model) => ({
            ...model,
            languages: withLanguageFlags(model.languages ?? []),
          })),
        );
        return;
      }
      setSonioxVoicesLoading(true);
      try {
        const preferred = sessionRef.current.sonioxTtsModel || "tts-rt-v1";
        const [rawModels, voices] = await Promise.all([
          listSonioxTtsModels(),
          listSonioxVoices(undefined, preferred),
        ]);
        const models = rawModels.map((model) => ({
          ...model,
          languages: withLanguageFlags(model.languages ?? []),
        }));
        if (models.length > 0) setSonioxTtsModels(models);
        if (voices.length > 0) {
          setSonioxVoices(voices);
        } else {
          setSonioxVoices(FALLBACK_SONIOX_VOICES);
          if (!opts?.silent) onToast("error", "No Soniox TTS voices returned");
        }
        await onSave(
          buildVoiceSave({
            sonioxTtsModels: models.length > 0 ? models : undefined,
            sonioxTtsVoices: voices.length > 0 ? voices : undefined,
            skipSonioxTtsModel: true,
          }),
        );
        if (!opts?.silent && (models.length > 0 || voices.length > 0)) {
          onToast("success", "Soniox TTS catalog updated");
        }
      } catch (e) {
        setSonioxVoices(
          (snapshot.sonioxTtsVoices?.length ?? 0) > 0
            ? (snapshot.sonioxTtsVoices ?? FALLBACK_SONIOX_VOICES)
            : FALLBACK_SONIOX_VOICES,
        );
        setSonioxTtsModels(
          (snapshot.sonioxTtsModels ?? []).map((model) => ({
            ...model,
            languages: withLanguageFlags(model.languages ?? []),
          })),
        );
        onToast("error", String(e));
      } finally {
        setSonioxVoicesLoading(false);
      }
    },
    [buildVoiceSave, listSonioxVoices, onSave, onToast],
  );

  useEffect(() => {
    if (config.aiProvider !== "soniox") return;
    const cachedVoices = config.sonioxTtsVoices ?? [];
    const cachedModels = config.sonioxTtsModels ?? [];
    if (cachedVoices.length > 0) setSonioxVoices(cachedVoices);
    if (cachedModels.length > 0) setSonioxTtsModels(cachedModels);
    if (cachedVoices.length > 0 && cachedModels.length > 0) return;
    if (!config.sonioxApiKeyConfigured) {
      if (cachedVoices.length === 0) setSonioxVoices(FALLBACK_SONIOX_VOICES);
      return;
    }
    void refreshSonioxVoices({ silent: true });
  }, [
    config.aiProvider,
    config.sonioxApiKeyConfigured,
    config.sonioxTtsVoices,
    config.sonioxTtsModels,
    refreshSonioxVoices,
  ]);

  useEffect(() => {
    onStateChange?.({ dirty });
  }, [dirty, onStateChange]);

  useEffect(() => {
    if (focusTarget !== "customVoice") return;
    (
      inboundCustomVoiceSectionRef.current ?? outboundCustomVoiceSectionRef.current
    )?.scrollIntoView({
      behavior: "smooth",
      block: "nearest",
    });
  }, [focusTarget]);

  const resetCustomVoiceSettings = useCallback(() => {
    setTtsModel(config.elevenlabsTtsModel ?? "eleven_flash_v2_5");
    setStability(config.elevenlabsStability ?? 0.5);
    setSimilarityBoost(config.elevenlabsSimilarityBoost ?? 0.75);
    setSynthesisMode(config.elevenlabsTtsSynthesisMode ?? "streaming");
  }, [config]);

  const resetInboundCustomVoiceSettings = useCallback(() => {
    setInboundTtsModel(
      config.elevenlabsInboundTtsModel ?? "eleven_flash_v2_5",
    );
    setInboundStability(config.elevenlabsInboundStability ?? 0.5);
    setInboundSimilarityBoost(
      config.elevenlabsInboundSimilarityBoost ?? 0.75,
    );
    setInboundSynthesisMode(
      config.elevenlabsInboundTtsSynthesisMode ?? "streaming",
    );
  }, [config]);

  const persistMode = useCallback(
    async (nextMode: OutboundVoiceOutput) => {
      const normalized = normalizeVoiceOutput(nextMode);
      if (normalized === normalizeVoiceOutput(config.outboundVoiceOutput)) return;
      setOutputMode(normalized);
      sessionRef.current.outputMode = normalized;
      setModeSaving(true);
      try {
        await onSave(buildVoiceSave({ outboundVoiceOutput: normalized }));
        onToast("success", "Voice output mode saved");
      } catch (e) {
        const reverted = normalizeVoiceOutput(config.outboundVoiceOutput);
        setOutputMode(reverted);
        sessionRef.current.outputMode = reverted;
        onToast("error", e instanceof Error ? e.message : "Failed to save");
      } finally {
        setModeSaving(false);
      }
    },
    [buildVoiceSave, config.outboundVoiceOutput, onSave, onToast],
  );

  const persistInboundMode = useCallback(
    async (nextMode: InboundVoiceOutput) => {
      const normalized = normalizeVoiceOutput(nextMode);
      if (normalized === normalizeVoiceOutput(config.inboundVoiceOutput)) return;
      setInboundOutputMode(normalized);
      sessionRef.current.inboundOutputMode = normalized;
      setInboundModeSaving(true);
      try {
        await onSave(buildVoiceSave({ inboundVoiceOutput: normalized }));
        onToast("success", "Meeting voice output mode saved");
      } catch (e) {
        const reverted = normalizeVoiceOutput(config.inboundVoiceOutput);
        setInboundOutputMode(reverted);
        sessionRef.current.inboundOutputMode = reverted;
        onToast("error", e instanceof Error ? e.message : "Failed to save");
      } finally {
        setInboundModeSaving(false);
      }
    },
    [buildVoiceSave, config.inboundVoiceOutput, onSave, onToast],
  );

  const persistOutboundVendor = useCallback(
    async (vendor: CustomVoiceVendor) => {
      const normalized = normalizeCustomVoiceVendor(vendor);
      if (normalized === normalizeCustomVoiceVendor(config.outboundCustomVoiceVendor)) {
        return;
      }
      setOutboundCustomVoiceVendor(normalized);
      sessionRef.current.outboundCustomVoiceVendor = normalized;
      setOutboundVendorSaving(true);
      try {
        await onSave(buildVoiceSave({ outboundCustomVoiceVendor: normalized }));
        onToast("success", "Custom voice engine saved");
      } catch (e) {
        const reverted = normalizeCustomVoiceVendor(config.outboundCustomVoiceVendor);
        setOutboundCustomVoiceVendor(reverted);
        sessionRef.current.outboundCustomVoiceVendor = reverted;
        onToast("error", e instanceof Error ? e.message : "Failed to save");
      } finally {
        setOutboundVendorSaving(false);
      }
    },
    [buildVoiceSave, config.outboundCustomVoiceVendor, onSave, onToast],
  );

  const persistInboundVendor = useCallback(
    async (vendor: CustomVoiceVendor) => {
      const normalized = normalizeCustomVoiceVendor(vendor);
      if (normalized === normalizeCustomVoiceVendor(config.inboundCustomVoiceVendor)) {
        return;
      }
      setInboundCustomVoiceVendor(normalized);
      sessionRef.current.inboundCustomVoiceVendor = normalized;
      setInboundVendorSaving(true);
      try {
        await onSave(buildVoiceSave({ inboundCustomVoiceVendor: normalized }));
        onToast("success", "Custom voice engine saved");
      } catch (e) {
        const reverted = normalizeCustomVoiceVendor(config.inboundCustomVoiceVendor);
        setInboundCustomVoiceVendor(reverted);
        sessionRef.current.inboundCustomVoiceVendor = reverted;
        onToast("error", e instanceof Error ? e.message : "Failed to save");
      } finally {
        setInboundVendorSaving(false);
      }
    },
    [buildVoiceSave, config.inboundCustomVoiceVendor, onSave, onToast],
  );

  const handleKeySaved = useCallback(() => {
    setVoicesNonce((n) => n + 1);
  }, []);

  const persistCustomVoiceSettings = useCallback(async () => {
    setCustomVoiceSettingsSaving(true);
    const needsOutboundRestart =
      speakingStyleAvailable &&
      synthesisMode !== (config.elevenlabsTtsSynthesisMode ?? "streaming");
    try {
      await onSave(
        buildVoiceSave({
          elevenlabsTtsModel: ttsModel,
          elevenlabsStability: stability,
          elevenlabsSimilarityBoost: similarityBoost,
          ...(speakingStyleAvailable
            ? { elevenlabsTtsSynthesisMode: synthesisMode }
            : {}),
        }),
      );
      onToast("success", "Voice settings saved");
      if (needsOutboundRestart && customVoiceEnabled) {
        onToast(
          "info",
          "Stop and Start outbound translation to apply Speaking style changes.",
        );
      }
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Failed to save");
    } finally {
      setCustomVoiceSettingsSaving(false);
    }
  }, [
    buildVoiceSave,
    customVoiceEnabled,
    config.elevenlabsTtsSynthesisMode,
    onSave,
    onToast,
    similarityBoost,
    speakingStyleAvailable,
    stability,
    synthesisMode,
    ttsModel,
  ]);

  const persistInboundCustomVoiceSettings = useCallback(async () => {
    setInboundCustomVoiceSettingsSaving(true);
    const needsInboundRestart =
      speakingStyleAvailable &&
      inboundSynthesisMode !==
        (config.elevenlabsInboundTtsSynthesisMode ?? "streaming");
    try {
      await onSave(
        buildVoiceSave({
          elevenlabsInboundTtsModel: inboundTtsModel,
          elevenlabsInboundStability: inboundStability,
          elevenlabsInboundSimilarityBoost: inboundSimilarityBoost,
          ...(speakingStyleAvailable
            ? { elevenlabsInboundTtsSynthesisMode: inboundSynthesisMode }
            : {}),
        }),
      );
      onToast("success", "Meeting voice settings saved");
      if (needsInboundRestart && inboundCustomVoiceEnabled) {
        onToast(
          "info",
          "Stop and Start inbound translation to apply Speaking style changes.",
        );
      }
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Failed to save");
    } finally {
      setInboundCustomVoiceSettingsSaving(false);
    }
  }, [
    buildVoiceSave,
    config.elevenlabsInboundTtsSynthesisMode,
    inboundCustomVoiceEnabled,
    inboundSimilarityBoost,
    inboundStability,
    inboundSynthesisMode,
    inboundTtsModel,
    onSave,
    onToast,
    speakingStyleAvailable,
  ]);

  const resetFishCustomVoiceSettings = useCallback(() => {
    setFishTtsModel(config.fishaudioTtsModel ?? "s2.1-pro");
    setFishLatency(config.fishaudioLatency ?? "balanced");
    setFishTemperature(config.fishaudioTemperature ?? 0.7);
    setFishSpeed(config.fishaudioSpeed ?? 1.0);
    setFishTopP(config.fishaudioTopP ?? 0.7);
  }, [config]);

  const resetInboundFishCustomVoiceSettings = useCallback(() => {
    setInboundFishTtsModel(config.fishaudioInboundTtsModel ?? "s2.1-pro");
    setInboundFishLatency(config.fishaudioInboundLatency ?? "balanced");
    setInboundFishTemperature(config.fishaudioInboundTemperature ?? 0.7);
    setFishSpeed(config.fishaudioSpeed ?? 1.0);
    setFishTopP(config.fishaudioTopP ?? 0.7);
  }, [config]);

  const persistFishCustomVoiceSettings = useCallback(async () => {
    setCustomVoiceSettingsSaving(true);
    try {
      await onSave(
        buildVoiceSave({
          fishaudioTtsModel: fishTtsModel,
          fishaudioLatency: fishLatency,
          fishaudioTemperature: fishTemperature,
          fishaudioSpeed: fishSpeed,
          fishaudioTopP: fishTopP,
        }),
      );
      onToast("success", "Voice settings saved");
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Failed to save");
    } finally {
      setCustomVoiceSettingsSaving(false);
    }
  }, [
    buildVoiceSave,
    fishLatency,
    fishSpeed,
    fishTemperature,
    fishTopP,
    fishTtsModel,
    onSave,
    onToast,
  ]);

  const persistInboundFishCustomVoiceSettings = useCallback(async () => {
    setInboundCustomVoiceSettingsSaving(true);
    try {
      await onSave(
        buildVoiceSave({
          fishaudioInboundTtsModel: inboundFishTtsModel,
          fishaudioInboundLatency: inboundFishLatency,
          fishaudioInboundTemperature: inboundFishTemperature,
          fishaudioSpeed: fishSpeed,
          fishaudioTopP: fishTopP,
        }),
      );
      onToast("success", "Meeting voice settings saved");
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Failed to save");
    } finally {
      setInboundCustomVoiceSettingsSaving(false);
    }
  }, [
    buildVoiceSave,
    fishSpeed,
    fishTopP,
    inboundFishLatency,
    inboundFishTemperature,
    inboundFishTtsModel,
    onSave,
    onToast,
  ]);

  return (
    <>
      <SettingsSection
        id="settings-section-voice-inbound"
        title="Meeting → You"
        status={status}
        focusKey={focusKey}
        activeFocus={activeFocus}
      >
        <p className="m-0 text-xs leading-relaxed text-muted-foreground">
          {inboundVoiceNote(config.aiProvider)}
        </p>
        <InboundVoiceModeSelect
          outputMode={inboundOutputMode}
          inboundLocked={inboundLocked}
          modeSaving={inboundModeSaving}
          onPersistMode={(mode) => void persistInboundMode(mode)}
        />
        {config.aiProvider === "soniox" &&
        inboundOutputMode === "providerNative" ? (
          <SonioxInboundVoiceSettings
            config={config}
            inboundLocked={inboundLocked}
            ttsModels={sonioxTtsModels}
            sonioxVoices={sonioxVoices}
            catalogLoading={sonioxVoicesLoading}
            showSharedModel
            onRefreshCatalog={() => void refreshSonioxVoices()}
            onPreviewVoice={previewSonioxVoice}
            onSave={saveVoicePatch}
            onToast={onToast}
          />
        ) : config.aiProvider !== "soniox" &&
          inboundOutputMode === "providerNative" ? (
          <p className="m-0 text-sm text-muted-foreground">
            From live session — no separate voice picker for this engine.
          </p>
        ) : null}
        {inboundCustomVoiceEnabled ? (
          <>
            <CustomVoiceVendorSelect
              id="inbound-custom-voice-vendor"
              vendor={inboundCustomVoiceVendor}
              locked={inboundLocked}
              saving={inboundVendorSaving}
              onPersist={(vendor) => void persistInboundVendor(vendor)}
            />
            {inboundUsesEl ? (
              <ElevenLabsCustomVoicePanel
                config={config}
                direction="inbound"
                locked={inboundLocked}
                apiKeyLocked={inboundLocked || outboundLocked}
                showApiKey
                customVoiceSectionRef={inboundCustomVoiceSectionRef}
                voicesNonce={voicesNonce}
                ttsModel={inboundTtsModel}
                setTtsModel={setInboundTtsModel}
                stability={inboundStability}
                setStability={setInboundStability}
                similarityBoost={inboundSimilarityBoost}
                setSimilarityBoost={setInboundSimilarityBoost}
                synthesisMode={inboundSynthesisMode}
                setSynthesisMode={setInboundSynthesisMode}
                customVoiceSettingsDirty={inboundCustomVoiceSettingsDirty}
                customVoiceSettingsSaving={inboundCustomVoiceSettingsSaving}
                onSave={saveKeepingCloneSession}
                onTestElevenLabs={testElevenLabsApiKey}
                onListElevenLabsVoices={listElevenLabsVoices}
                onListElevenLabsModels={listElevenLabsModels}
                onValidateElevenLabsVoice={validateElevenLabsVoice}
                onPreviewElevenLabsVoice={previewElevenLabsVoice}
                onToast={onToast}
                onKeyDirty={setKeyDirty}
                onVoiceDirty={setVoiceDirty}
                onKeySaved={handleKeySaved}
                persistElevenLabsVoices={persistElevenLabsVoices}
                persistElevenLabsModels={persistElevenLabsModels}
                resetCustomVoiceSettings={resetInboundCustomVoiceSettings}
                persistCustomVoiceSettings={() => void persistInboundCustomVoiceSettings()}
              />
            ) : (
              <FishAudioCustomVoicePanel
                config={config}
                direction="inbound"
                locked={inboundLocked}
                apiKeyLocked={inboundLocked || outboundLocked}
                showApiKey
                customVoiceSectionRef={inboundCustomVoiceSectionRef}
                voicesNonce={voicesNonce}
                ttsModel={inboundFishTtsModel}
                setTtsModel={setInboundFishTtsModel}
                latency={inboundFishLatency}
                setLatency={setInboundFishLatency}
                temperature={inboundFishTemperature}
                setTemperature={setInboundFishTemperature}
                speed={fishSpeed}
                setSpeed={setFishSpeed}
                topP={fishTopP}
                setTopP={setFishTopP}
                customVoiceSettingsDirty={inboundFishCustomVoiceSettingsDirty}
                customVoiceSettingsSaving={inboundCustomVoiceSettingsSaving}
                onSave={saveKeepingCloneSession}
                onTestFishAudio={testFishAudioApiKey}
                onListFishAudioVoices={listFishAudioVoices}
                onListFishAudioModels={listFishAudioModels}
                onValidateFishAudioVoice={validateFishAudioVoice}
                onPreviewFishAudioVoice={previewFishAudioVoice}
                onToast={onToast}
                onKeyDirty={setKeyDirty}
                onVoiceDirty={setVoiceDirty}
                onKeySaved={handleKeySaved}
                persistFishAudioVoices={persistFishAudioVoices}
                persistFishAudioModels={persistFishAudioModels}
                resetCustomVoiceSettings={resetInboundFishCustomVoiceSettings}
                persistCustomVoiceSettings={() => void persistInboundFishCustomVoiceSettings()}
              />
            )}
          </>
        ) : null}
      </SettingsSection>

      <SettingsSection
        id="settings-section-voice-outbound"
        title="You → Meeting"
      >
        <p className="m-0 text-xs leading-relaxed text-muted-foreground">
          {customVoiceEnabled
            ? "Custom voice for translated You → Meeting audio."
            : engineVoiceHint(config.aiProvider)}
        </p>
        <OutboundVoiceModeSelect
          config={config}
          outputMode={outputMode}
          customVoiceEnabled={customVoiceEnabled}
          outboundLocked={outboundLocked}
          modeSaving={modeSaving}
          onPersistMode={(mode) => void persistMode(mode)}
        />
        {config.aiProvider === "soniox" &&
        outputMode === "providerNative" ? (
          <SonioxEngineVoiceSettings
            config={config}
            direction="outbound"
            locked={outboundLocked}
            ttsModels={sonioxTtsModels}
            sonioxVoices={sonioxVoices}
            catalogLoading={sonioxVoicesLoading}
            showSharedModel
            onRefreshCatalog={() => void refreshSonioxVoices()}
            onPreviewVoice={previewSonioxVoice}
            onSave={saveVoicePatch}
            onToast={onToast}
          />
        ) : config.aiProvider !== "soniox" &&
          outputMode === "providerNative" ? (
          <p className="m-0 text-sm text-muted-foreground">
            From live session — no separate voice picker for this engine.
          </p>
        ) : null}
        {customVoiceEnabled ? (
          <>
            <CustomVoiceVendorSelect
              id="outbound-custom-voice-vendor"
              vendor={outboundCustomVoiceVendor}
              locked={outboundLocked}
              saving={outboundVendorSaving}
              onPersist={(vendor) => void persistOutboundVendor(vendor)}
            />
            {outboundUsesEl ? (
              <ElevenLabsCustomVoicePanel
                config={config}
                direction="outbound"
                locked={outboundLocked}
                apiKeyLocked={inboundLocked || outboundLocked}
                showApiKey={!inboundUsesEl}
                customVoiceSectionRef={outboundCustomVoiceSectionRef}
                voicesNonce={voicesNonce}
                ttsModel={ttsModel}
                setTtsModel={setTtsModel}
                stability={stability}
                setStability={setStability}
                similarityBoost={similarityBoost}
                setSimilarityBoost={setSimilarityBoost}
                synthesisMode={synthesisMode}
                setSynthesisMode={setSynthesisMode}
                customVoiceSettingsDirty={customVoiceSettingsDirty}
                customVoiceSettingsSaving={customVoiceSettingsSaving}
                onSave={saveKeepingCloneSession}
                onTestElevenLabs={testElevenLabsApiKey}
                onListElevenLabsVoices={listElevenLabsVoices}
                onListElevenLabsModels={listElevenLabsModels}
                onValidateElevenLabsVoice={validateElevenLabsVoice}
                onPreviewElevenLabsVoice={previewElevenLabsVoice}
                onToast={onToast}
                onKeyDirty={setKeyDirty}
                onVoiceDirty={setVoiceDirty}
                onKeySaved={handleKeySaved}
                persistElevenLabsVoices={persistElevenLabsVoices}
                persistElevenLabsModels={persistElevenLabsModels}
                resetCustomVoiceSettings={resetCustomVoiceSettings}
                persistCustomVoiceSettings={() => void persistCustomVoiceSettings()}
              />
            ) : (
              <FishAudioCustomVoicePanel
                config={config}
                direction="outbound"
                locked={outboundLocked}
                apiKeyLocked={inboundLocked || outboundLocked}
                showApiKey={!inboundUsesFish}
                customVoiceSectionRef={outboundCustomVoiceSectionRef}
                voicesNonce={voicesNonce}
                ttsModel={fishTtsModel}
                setTtsModel={setFishTtsModel}
                latency={fishLatency}
                setLatency={setFishLatency}
                temperature={fishTemperature}
                setTemperature={setFishTemperature}
                speed={fishSpeed}
                setSpeed={setFishSpeed}
                topP={fishTopP}
                setTopP={setFishTopP}
                customVoiceSettingsDirty={fishCustomVoiceSettingsDirty}
                customVoiceSettingsSaving={customVoiceSettingsSaving}
                onSave={saveKeepingCloneSession}
                onTestFishAudio={testFishAudioApiKey}
                onListFishAudioVoices={listFishAudioVoices}
                onListFishAudioModels={listFishAudioModels}
                onValidateFishAudioVoice={validateFishAudioVoice}
                onPreviewFishAudioVoice={previewFishAudioVoice}
                onToast={onToast}
                onKeyDirty={setKeyDirty}
                onVoiceDirty={setVoiceDirty}
                onKeySaved={handleKeySaved}
                persistFishAudioVoices={persistFishAudioVoices}
                persistFishAudioModels={persistFishAudioModels}
                resetCustomVoiceSettings={resetFishCustomVoiceSettings}
                persistCustomVoiceSettings={() => void persistFishCustomVoiceSettings()}
              />
            )}
          </>
        ) : null}
      </SettingsSection>
    </>
  );
}
