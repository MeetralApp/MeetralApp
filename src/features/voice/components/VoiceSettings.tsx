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
import type { ConfigView, ElevenLabsModelOption, ElevenLabsVoiceOption, InboundVoiceOutput, OutboundVoiceOutput, SonioxVoiceOption, TtsSynthesisMode, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";

import ElevenLabsCloneSettingsPanel from "./ElevenLabsCloneSettingsPanel";
import InboundVoiceModeSelect from "./InboundVoiceModeSelect";
import OutboundVoiceModeSelect from "./OutboundVoiceModeSelect";
import SonioxEngineVoiceSettings from "./SonioxEngineVoiceSettings";
import SonioxInboundVoiceSettings from "./SonioxInboundVoiceSettings";
export interface VoiceSettingsState {
  dirty: boolean;
}

interface Props {
  config: ConfigView;
  outboundLocked: boolean;
  inboundLocked: boolean;
  status?: SettingsSectionStatus;
  focusKey?: SettingsFocus;
  activeFocus?: SettingsFocus | null;
  /** Deep-link into clone / ElevenLabs block when Settings opens with focus `clone`. */
  focusTarget?: "clone";
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
  } = useVoiceCatalog();
  const [inboundOutputMode, setInboundOutputMode] =
    useState<InboundVoiceOutput>(
      config.inboundVoiceOutput ?? "providerNative",
    );
  const [outputMode, setOutputMode] = useState<OutboundVoiceOutput>(
    config.outboundVoiceOutput ?? "providerNative",
  );
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
  const [inboundCloneSettingsSaving, setInboundCloneSettingsSaving] =
    useState(false);
  const [cloneSettingsSaving, setCloneSettingsSaving] = useState(false);
  const inboundCloneSectionRef = useRef<HTMLDivElement>(null);
  const outboundCloneSectionRef = useRef<HTMLDivElement>(null);

  const speakingStyleAvailable = config.aiProvider !== "soniox";
  const cloneSettingsDirty =
    ttsModel !== (config.elevenlabsTtsModel ?? "eleven_flash_v2_5") ||
    stability !== (config.elevenlabsStability ?? 0.5) ||
    similarityBoost !== (config.elevenlabsSimilarityBoost ?? 0.75) ||
    (speakingStyleAvailable &&
      synthesisMode !== (config.elevenlabsTtsSynthesisMode ?? "streaming"));

  const inboundCloneSettingsDirty =
    inboundTtsModel !==
      (config.elevenlabsInboundTtsModel ?? "eleven_flash_v2_5") ||
    inboundStability !== (config.elevenlabsInboundStability ?? 0.5) ||
    inboundSimilarityBoost !==
      (config.elevenlabsInboundSimilarityBoost ?? 0.75) ||
    (speakingStyleAvailable &&
      inboundSynthesisMode !==
        (config.elevenlabsInboundTtsSynthesisMode ?? "streaming"));

  const inboundCloneEnabled = inboundOutputMode === "elevenLabsClone";
  const cloneEnabled = outputMode === "elevenLabsClone";
  const dirty =
    keyDirty ||
    voiceDirty ||
    (inboundCloneEnabled && inboundCloneSettingsDirty) ||
    (cloneEnabled && cloneSettingsDirty);

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
    setInboundOutputMode(
      config.inboundVoiceOutput ?? "providerNative",
    );
  }, [config.inboundVoiceOutput]);

  useEffect(() => {
    setOutputMode(config.outboundVoiceOutput ?? "providerNative");
  }, [config.outboundVoiceOutput]);

  const persistElevenLabsVoices = useCallback(
    async (list: ElevenLabsVoiceOption[]) => {
      await onSave(toSavePayload(config, { elevenlabsVoices: list }));
    },
    [config, onSave],
  );

  const persistElevenLabsModels = useCallback(
    async (list: ElevenLabsModelOption[]) => {
      await onSave(toSavePayload(config, { elevenlabsModels: list }));
    },
    [config, onSave],
  );

  const refreshSonioxVoices = useCallback(
    async (opts?: { silent?: boolean }) => {
      if (!config.sonioxApiKeyConfigured) {
        setSonioxVoices(
          (config.sonioxTtsVoices?.length ?? 0) > 0
            ? (config.sonioxTtsVoices ?? FALLBACK_SONIOX_VOICES)
            : FALLBACK_SONIOX_VOICES,
        );
        setSonioxTtsModels(
          (config.sonioxTtsModels ?? []).map((model) => ({
            ...model,
            languages: withLanguageFlags(model.languages ?? []),
          })),
        );
        return;
      }
      setSonioxVoicesLoading(true);
      try {
        const preferred = config.sonioxTtsModel ?? "tts-rt-v1";
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
          toSavePayload(config, {
            sonioxTtsModels: models.length > 0 ? models : undefined,
            sonioxTtsVoices: voices.length > 0 ? voices : undefined,
          }),
        );
        if (!opts?.silent && (models.length > 0 || voices.length > 0)) {
          onToast("success", "Soniox TTS catalog updated");
        }
      } catch (e) {
        setSonioxVoices(
          (config.sonioxTtsVoices?.length ?? 0) > 0
            ? (config.sonioxTtsVoices ?? FALLBACK_SONIOX_VOICES)
            : FALLBACK_SONIOX_VOICES,
        );
        setSonioxTtsModels(
          (config.sonioxTtsModels ?? []).map((model) => ({
            ...model,
            languages: withLanguageFlags(model.languages ?? []),
          })),
        );
        onToast("error", String(e));
      } finally {
        setSonioxVoicesLoading(false);
      }
    },
    [config, listSonioxVoices, onSave, onToast],
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
    if (focusTarget !== "clone") return;
    (
      inboundCloneSectionRef.current ?? outboundCloneSectionRef.current
    )?.scrollIntoView({
      behavior: "smooth",
      block: "nearest",
    });
  }, [focusTarget]);

  const resetCloneSettings = useCallback(() => {
    setTtsModel(config.elevenlabsTtsModel ?? "eleven_flash_v2_5");
    setStability(config.elevenlabsStability ?? 0.5);
    setSimilarityBoost(config.elevenlabsSimilarityBoost ?? 0.75);
    setSynthesisMode(config.elevenlabsTtsSynthesisMode ?? "streaming");
  }, [config]);

  const resetInboundCloneSettings = useCallback(() => {
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
      if (nextMode === (config.outboundVoiceOutput ?? "providerNative")) return;
      setOutputMode(nextMode);
      setModeSaving(true);
      try {
        await onSave(
          toSavePayload(config, { outboundVoiceOutput: nextMode }),
        );
        onToast("success", "Voice output mode saved");
      } catch (e) {
        setOutputMode(config.outboundVoiceOutput ?? "providerNative");
        onToast("error", e instanceof Error ? e.message : "Failed to save");
      } finally {
        setModeSaving(false);
      }
    },
    [config, onSave, onToast],
  );

  const persistInboundMode = useCallback(
    async (nextMode: InboundVoiceOutput) => {
      if (nextMode === (config.inboundVoiceOutput ?? "providerNative")) return;
      setInboundOutputMode(nextMode);
      setInboundModeSaving(true);
      try {
        await onSave(
          toSavePayload(config, { inboundVoiceOutput: nextMode }),
        );
        onToast("success", "Meeting voice output mode saved");
      } catch (e) {
        setInboundOutputMode(
          config.inboundVoiceOutput ?? "providerNative",
        );
        onToast("error", e instanceof Error ? e.message : "Failed to save");
      } finally {
        setInboundModeSaving(false);
      }
    },
    [config, onSave, onToast],
  );

  const handleKeySaved = useCallback(() => {
    setVoicesNonce((n) => n + 1);
  }, []);

  const persistCloneSettings = useCallback(async () => {
    setCloneSettingsSaving(true);
    const needsOutboundRestart =
      speakingStyleAvailable &&
      synthesisMode !== (config.elevenlabsTtsSynthesisMode ?? "streaming");
    try {
      await onSave(
        toSavePayload(config, {
          elevenlabsTtsModel: ttsModel,
          elevenlabsStability: stability,
          elevenlabsSimilarityBoost: similarityBoost,
          ...(speakingStyleAvailable
            ? { elevenlabsTtsSynthesisMode: synthesisMode }
            : {}),
        }),
      );
      onToast("success", "Voice settings saved");
      if (needsOutboundRestart && cloneEnabled) {
        onToast(
          "info",
          "Stop and Start outbound translation to apply Speaking style changes.",
        );
      }
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Failed to save");
    } finally {
      setCloneSettingsSaving(false);
    }
  }, [
    cloneEnabled,
    config,
    onSave,
    onToast,
    similarityBoost,
    speakingStyleAvailable,
    stability,
    synthesisMode,
    ttsModel,
  ]);

  const persistInboundCloneSettings = useCallback(async () => {
    setInboundCloneSettingsSaving(true);
    const needsInboundRestart =
      speakingStyleAvailable &&
      inboundSynthesisMode !==
        (config.elevenlabsInboundTtsSynthesisMode ?? "streaming");
    try {
      await onSave(
        toSavePayload(config, {
          elevenlabsInboundTtsModel: inboundTtsModel,
          elevenlabsInboundStability: inboundStability,
          elevenlabsInboundSimilarityBoost: inboundSimilarityBoost,
          ...(speakingStyleAvailable
            ? { elevenlabsInboundTtsSynthesisMode: inboundSynthesisMode }
            : {}),
        }),
      );
      onToast("success", "Meeting voice settings saved");
      if (needsInboundRestart && inboundCloneEnabled) {
        onToast(
          "info",
          "Stop and Start inbound translation to apply Speaking style changes.",
        );
      }
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Failed to save");
    } finally {
      setInboundCloneSettingsSaving(false);
    }
  }, [
    config,
    inboundCloneEnabled,
    inboundSimilarityBoost,
    inboundStability,
    inboundSynthesisMode,
    inboundTtsModel,
    onSave,
    onToast,
    speakingStyleAvailable,
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
            onSave={onSave}
            onToast={onToast}
          />
        ) : config.aiProvider !== "soniox" &&
          inboundOutputMode === "providerNative" ? (
          <p className="m-0 text-sm text-muted-foreground">
            From live session — no separate voice picker for this engine.
          </p>
        ) : null}
        {inboundCloneEnabled ? (
          <ElevenLabsCloneSettingsPanel
            config={config}
            direction="inbound"
            locked={inboundLocked}
            apiKeyLocked={inboundLocked || outboundLocked}
            showApiKey
            cloneSectionRef={inboundCloneSectionRef}
            voicesNonce={voicesNonce}
            ttsModel={inboundTtsModel}
            setTtsModel={setInboundTtsModel}
            stability={inboundStability}
            setStability={setInboundStability}
            similarityBoost={inboundSimilarityBoost}
            setSimilarityBoost={setInboundSimilarityBoost}
            synthesisMode={inboundSynthesisMode}
            setSynthesisMode={setInboundSynthesisMode}
            cloneSettingsDirty={inboundCloneSettingsDirty}
            cloneSettingsSaving={inboundCloneSettingsSaving}
            onSave={onSave}
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
            resetCloneSettings={resetInboundCloneSettings}
            persistCloneSettings={() => void persistInboundCloneSettings()}
          />
        ) : null}
      </SettingsSection>

      <SettingsSection
        id="settings-section-voice-outbound"
        title="You → Meeting"
      >
        <p className="m-0 text-xs leading-relaxed text-muted-foreground">
          {cloneEnabled
            ? "ElevenLabs voice for translated You → Meeting audio."
            : engineVoiceHint(config.aiProvider)}
        </p>
        <OutboundVoiceModeSelect
          config={config}
          outputMode={outputMode}
          cloneEnabled={cloneEnabled}
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
            showSharedModel={inboundOutputMode !== "providerNative"}
            onRefreshCatalog={() => void refreshSonioxVoices()}
            onPreviewVoice={previewSonioxVoice}
            onSave={onSave}
            onToast={onToast}
          />
        ) : config.aiProvider !== "soniox" &&
          outputMode === "providerNative" ? (
          <p className="m-0 text-sm text-muted-foreground">
            From live session — no separate voice picker for this engine.
          </p>
        ) : null}
        {cloneEnabled ? (
          <ElevenLabsCloneSettingsPanel
            config={config}
            direction="outbound"
            locked={outboundLocked}
            apiKeyLocked={inboundLocked || outboundLocked}
            showApiKey={!inboundCloneEnabled}
            cloneSectionRef={outboundCloneSectionRef}
            voicesNonce={voicesNonce}
            ttsModel={ttsModel}
            setTtsModel={setTtsModel}
            stability={stability}
            setStability={setStability}
            similarityBoost={similarityBoost}
            setSimilarityBoost={setSimilarityBoost}
            synthesisMode={synthesisMode}
            setSynthesisMode={setSynthesisMode}
            cloneSettingsDirty={cloneSettingsDirty}
            cloneSettingsSaving={cloneSettingsSaving}
            onSave={onSave}
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
            resetCloneSettings={resetCloneSettings}
            persistCloneSettings={() => void persistCloneSettings()}
          />
        ) : null}
      </SettingsSection>
    </>
  );
}
