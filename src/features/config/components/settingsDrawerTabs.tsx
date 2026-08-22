import AdvancedSettings from "./AdvancedSettings";
import AboutFooter from "./AboutFooter";
import ApiKeySettings from "@/features/ai/components/ApiKeySettings";
import AiModelSettings from "@/features/ai/components/AiModelSettings";
import AudioDeviceSettings from "@/features/audio/components/AudioDeviceSettings";
import LanguageSettings from "./LanguageSettings";
import VoiceSettings from "@/features/voice/components/VoiceSettings";
import SonioxContextSettings from "@/features/voice/components/soniox-context/SonioxContextSettings";
import IntelligenceSettings from "@/features/config/components/IntelligenceSettings";
import { selectionUsable } from "@/features/config/lib/summaryProvider";
import SpeechDetectionSettings from "./SpeechDetectionSettings";
import ProviderSettings from "@/features/ai/components/ProviderSettings";
import TranscriptDisplaySettings from "./TranscriptDisplaySettings";
import OverlaySettings from "@/features/overlay/components/OverlaySettings";
import SettingsSection from "./SettingsSection";
import ApiKeyChip from "./ApiKeyChip";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SettingsField from "@/shared/components/SettingsField";
import SettingsIconButton from "@/shared/components/SettingsIconButton";
import {
  ApiKeyTitleHint,
  EngineTitleHint,
  LiveModelTitleHint,
  SonioxContextTitleHint,
} from "./settingsDrawerHints";
import { cn } from "@/shared/lib/utils";
import { Label } from "@/shared/ui/label";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { useLiveModelCatalog } from "@/features/ai/hooks/useLiveModelCatalog";
import { useAiCatalog } from "@/features/ai/hooks/useAiCatalog";
import { isNotesSession } from "@/features/pipeline/lib/sessionMode";
import type { ToastType } from "@/shared/context/toastTypes";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import type { AudioSetupValidation } from "@/features/audio/lib/audioSetup";
import type { SettingsFocus } from "@/features/config/lib/settingsFocus";
import type { ConfigView, DevicesResponse, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import type { SettingsSectionStatus } from "@/features/config/lib/settingsDrawerStatus";
import { RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";

type ToastFn = (type: ToastType, text: string) => void;

const ENGINE_API_KEY_PANEL_ID = "engine-api-key-panel";

export function TranslateTabPanel({
  config,
  languagesLocked,
  effectiveFocus,
  translationStatus,
  sonioxContextStatus,
  onSave,
  onSaveLanguages,
  onTestApiKey,
  onToast,
  onApiKeyDirty,
  onSonioxContextDirty,
  onSpeechDetectionDirty,
}: {
  config: ConfigView;
  languagesLocked: boolean;
  effectiveFocus: SettingsFocus | null | undefined;
  translationStatus: SettingsSectionStatus;
  sonioxContextStatus: SettingsSectionStatus | undefined;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onSaveLanguages: (myLanguage: string, meetingLanguage: string) => Promise<void>;
  onTestApiKey: (
    apiKey: string,
    provider?: AiProvider,
    purpose?: "live" | "summary",
  ) => Promise<void>;
  onToast: ToastFn;
  onApiKeyDirty: (dirty: boolean) => void;
  onSonioxContextDirty: (dirty: boolean) => void;
  onSpeechDetectionDirty: (dirty: boolean) => void;
}) {
  const [catalogNonce, setCatalogNonce] = useState(0);
  const [apiKeyPanelOpen, setApiKeyPanelOpen] = useState(false);
  const apiKeyWasDirty = useRef(false);
  const keyConfigured = config.apiKeyConfigured;
  const keyReady =
    config.aiProvider === "soniox"
      ? Boolean(config.sonioxApiKeyConfigured)
      : config.aiProvider === "openAi"
        ? Boolean(config.openaiApiKeyConfigured)
        : Boolean(config.geminiApiKeyConfigured ?? config.apiKeyConfigured);
  const prevKeyReady = useRef(keyReady);
  useEffect(() => {
    if (keyReady && !prevKeyReady.current) {
      setCatalogNonce((n) => n + 1);
    }
    prevKeyReady.current = keyReady;
  }, [keyReady]);

  useEffect(() => {
    if (!keyConfigured) {
      setApiKeyPanelOpen(false);
      apiKeyWasDirty.current = false;
    }
  }, [keyConfigured]);

  useEffect(() => {
    if (effectiveFocus !== "api") return;
    if (keyConfigured) {
      setApiKeyPanelOpen(true);
      document
        .getElementById("settings-section-provider")
        ?.scrollIntoView({ behavior: "smooth", block: "nearest" });
      return;
    }
    document
      .getElementById("settings-section-translation")
      ?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }, [effectiveFocus, keyConfigured]);

  useEffect(() => {
    if (effectiveFocus !== "languages") return;
    document
      .getElementById("settings-section-languages")
      ?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }, [effectiveFocus]);

  const { models, languages, loading, refresh } = useLiveModelCatalog(
    config,
    onSave,
    catalogNonce,
  );
  const providerCatalog = useAiCatalog(config.aiProvider);
  const notesOpenAiStt =
    isNotesSession(config) &&
    config.aiProvider === "openAi" &&
    Boolean(providerCatalog.notesSttModel);
  const [modelRefreshing, setModelRefreshing] = useState(false);
  const modelCatalogBusy = modelRefreshing || loading;

  const handleRefreshLiveModels = async () => {
    setModelRefreshing(true);
    try {
      await refresh();
      onToast("success", "Model catalog refreshed");
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setModelRefreshing(false);
    }
  };

  const handleApiKeyDirty = (dirty: boolean) => {
    onApiKeyDirty(dirty);
    if (dirty) {
      setApiKeyPanelOpen(true);
      apiKeyWasDirty.current = true;
      return;
    }
    if (apiKeyWasDirty.current && keyConfigured) {
      setApiKeyPanelOpen(false);
    }
    apiKeyWasDirty.current = false;
  };

  return (
    <>
      <SettingsSection
        id="settings-section-provider"
        focusKey="provider"
        activeFocus={effectiveFocus}
      >
        <SettingsField
          labelContent={
            <>
              <Label
                htmlFor="settings-ai-provider"
                className={settingsFieldLabelClass}
              >
                Engine
              </Label>
              <EngineTitleHint />
            </>
          }
          actions={
            keyConfigured ? (
              <ApiKeyChip
                open={apiKeyPanelOpen}
                status={translationStatus}
                panelId={ENGINE_API_KEY_PANEL_ID}
                onToggle={() => setApiKeyPanelOpen((open) => !open)}
              />
            ) : null
          }
        >
          <ProviderSettings
            config={config}
            locked={languagesLocked}
            onSave={onSave}
            onToast={onToast}
          />
          {keyConfigured && apiKeyPanelOpen ? (
            <div
              id={ENGINE_API_KEY_PANEL_ID}
              className="flex flex-col gap-3 pt-3"
            >
              <ApiKeySettings
                config={config}
                onSave={onSave}
                onTest={onTestApiKey}
                onToast={onToast}
                onStateChange={({ dirty }) => handleApiKeyDirty(dirty)}
              />
            </div>
          ) : null}
        </SettingsField>

        {!keyConfigured ? (
          <SettingsField
            labelContent={
              <>
                <span className={settingsFieldLabelClass}>API key</span>
                <ApiKeyTitleHint provider={config.aiProvider} />
              </>
            }
          >
            <div
              id="settings-section-translation"
              role="group"
              aria-label="API key"
            >
              <ApiKeySettings
                config={config}
                onSave={onSave}
                onTest={onTestApiKey}
                onToast={onToast}
                onStateChange={({ dirty }) => onApiKeyDirty(dirty)}
              />
            </div>
          </SettingsField>
        ) : null}

        <SettingsField
          labelContent={
            <>
              <Label
                htmlFor="settings-live-model"
                className={settingsFieldLabelClass}
              >
                {notesOpenAiStt ? "STT model" : "Live model"}
              </Label>
              <LiveModelTitleHint
                provider={config.aiProvider}
                sessionMode={config.sessionMode}
              />
            </>
          }
          actions={
            notesOpenAiStt ? null : (
              <SettingsIconButton
                label={modelCatalogBusy ? "Refreshing…" : "Refresh models"}
                icon={RefreshCw}
                disabled={modelCatalogBusy}
                busy={modelCatalogBusy}
                onClick={() => void handleRefreshLiveModels()}
              />
            )
          }
        >
          <div id="settings-section-models">
            <AiModelSettings
              config={config}
              models={models}
              loading={loading}
              onSave={onSave}
              onToast={onToast}
              notesSttModel={providerCatalog.notesSttModel}
            />
          </div>
        </SettingsField>

        <div id="settings-section-languages">
          <LanguageSettings
            config={config}
            languages={languages}
            languagesLocked={languagesLocked}
            onSave={onSaveLanguages}
          />
        </div>
      </SettingsSection>

      {config.aiProvider === "soniox" ? (
        <SettingsSection
          id="settings-section-soniox-extras"
          title="Context"
          titleHint={<SonioxContextTitleHint />}
          status={sonioxContextStatus}
          focusKey="sonioxContext"
          activeFocus={effectiveFocus}
        >
          <SonioxContextSettings
            config={config}
            onSave={onSave}
            onToast={onToast}
            onDirtyChange={onSonioxContextDirty}
          />
        </SettingsSection>
      ) : null}

      {config.aiProvider === "gemini" ? (
        <SettingsSection
          id="settings-section-speech-detection"
          title="Speech detection"
          titleHint={
            <SettingInfoHint label="About speech detection">
              Gemini-only controls for when speech starts and ends. Defaults
              work for most meetings.
            </SettingInfoHint>
          }
        >
          <SpeechDetectionSettings
            config={config}
            onSave={onSave}
            onToast={onToast}
            onDirtyChange={onSpeechDetectionDirty}
          />
        </SettingsSection>
      ) : null}
    </>
  );
}

export function IntelligenceTabPanel({
  config,
  intelligenceStatus,
  effectiveFocus,
  onSave,
  onTestApiKey,
  onToast,
  onApiKeyDirty,
}: {
  config: ConfigView;
  intelligenceStatus: SettingsSectionStatus;
  effectiveFocus: SettingsFocus | null | undefined;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTestApiKey: (
    apiKey: string,
    provider?: AiProvider,
    purpose?: "live" | "summary",
  ) => Promise<void>;
  onToast: ToastFn;
  onApiKeyDirty?: (dirty: boolean) => void;
}) {
  const [apiKeyPanelOpen, setApiKeyPanelOpen] = useState(false);
  const apiKeyWasDirty = useRef(false);
  // Custom local profiles count as configured (save-time test passed; auth
  // optional) — the API-key chip only applies to built-in providers.
  const hasKey = selectionUsable(config);

  useEffect(() => {
    if (!hasKey) {
      setApiKeyPanelOpen(false);
      apiKeyWasDirty.current = false;
    }
  }, [hasKey]);

  const handleApiKeyDirty = (dirty: boolean) => {
    onApiKeyDirty?.(dirty);
    if (dirty) {
      setApiKeyPanelOpen(true);
      apiKeyWasDirty.current = true;
      return;
    }
    if (apiKeyWasDirty.current && hasKey) {
      setApiKeyPanelOpen(false);
    }
    apiKeyWasDirty.current = false;
  };

  return (
    <SettingsSection
      id="settings-section-intelligence"
      status={hasKey ? undefined : intelligenceStatus}
      focusKey="intelligence"
      activeFocus={effectiveFocus}
    >
      <IntelligenceSettings
        config={config}
        onSave={onSave}
        onTestApiKey={(key, provider) => onTestApiKey(key, provider, "summary")}
        onToast={onToast}
        apiKeyPanelOpen={apiKeyPanelOpen}
        onApiKeyDirty={handleApiKeyDirty}
        apiKeyChip={
          hasKey ? (
            <ApiKeyChip
              open={apiKeyPanelOpen}
              status={intelligenceStatus}
              panelId="intelligence-api-key-panel"
              onToggle={() => setApiKeyPanelOpen((open) => !open)}
            />
          ) : null
        }
      />
    </SettingsSection>
  );
}

export function VoiceTabPanel({
  config,
  outboundLocked,
  inboundLocked,
  voiceStatus,
  voiceSectionFocus,
  effectiveFocus,
  onSave,
  onToast,
  onVoiceDirty,
}: {
  config: ConfigView;
  outboundLocked: boolean;
  inboundLocked: boolean;
  voiceStatus: SettingsSectionStatus;
  voiceSectionFocus: SettingsFocus | null | undefined;
  effectiveFocus: SettingsFocus | null | undefined;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: ToastFn;
  onVoiceDirty: (dirty: boolean) => void;
}) {
  return (
    <VoiceSettings
      config={config}
      outboundLocked={outboundLocked}
      inboundLocked={inboundLocked}
      status={voiceStatus}
      focusKey="voice"
      activeFocus={voiceSectionFocus}
      focusTarget={effectiveFocus === "clone" ? "clone" : undefined}
      onSave={onSave}
      onToast={onToast}
      onStateChange={({ dirty }) => onVoiceDirty(dirty)}
    />
  );
}

export function AudioTabPanel({
  config,
  devices,
  audioSetup,
  audioStatus,
  effectiveFocus,
  onSave,
  onRefreshDevices,
  onToast,
  onAudioDirty,
}: {
  config: ConfigView;
  devices: DevicesResponse | null;
  audioSetup: AudioSetupValidation | null;
  audioStatus: SettingsSectionStatus;
  effectiveFocus: SettingsFocus | null | undefined;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onRefreshDevices: () => Promise<DevicesResponse | void>;
  onToast: ToastFn;
  onAudioDirty: (dirty: boolean) => void;
}) {
  return (
    <SettingsSection
      id="settings-section-audio"
      title="Routing"
      status={audioStatus}
      focusKey="audio"
      activeFocus={effectiveFocus}
    >
      <AudioDeviceSettings
        config={config}
        devices={devices?.devices ?? []}
        validation={audioSetup}
        onSave={onSave}
        onRefreshDevices={onRefreshDevices}
        onToast={onToast}
        onDirtyChange={onAudioDirty}
      />
    </SettingsSection>
  );
}

export function AppTabPanel({
  config,
  appStatus,
  effectiveFocus,
  onSave,
  onToast,
  onAppDirty,
}: {
  config: ConfigView;
  appStatus: SettingsSectionStatus | undefined;
  effectiveFocus: SettingsFocus | null | undefined;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: ToastFn;
  onAppDirty: (dirty: boolean) => void;
}) {
  return (
    <>
      <SettingsSection
        id="settings-section-transcript-display"
        title="Transcript display"
      >
        <TranscriptDisplaySettings config={config} onSave={onSave} />
      </SettingsSection>

      <SettingsSection
        id="settings-section-overlay"
        title="Overlay"
        focusKey="overlay"
        activeFocus={effectiveFocus}
      >
        <OverlaySettings config={config} onSave={onSave} onToast={onToast} />
      </SettingsSection>

      <SettingsSection
        id="settings-section-preferences"
        title="Preferences"
        status={appStatus}
        focusKey="app"
        activeFocus={effectiveFocus}
      >
        <AdvancedSettings
          config={config}
          onSave={onSave}
          onToast={onToast}
          onDirtyChange={onAppDirty}
        />
      </SettingsSection>

      <div className={cn("pt-2")}>
        <AboutFooter />
      </div>
    </>
  );
}
