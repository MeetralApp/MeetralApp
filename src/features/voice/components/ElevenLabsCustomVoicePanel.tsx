import type { RefObject } from "react";
import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";

import SectionHeading from "@/shared/components/SectionHeading";
import SettingsGroup from "@/shared/components/SettingsGroup";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/shared/ui/collapsible";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import ApiKeyChip from "@/features/config/components/ApiKeyChip";
import ElevenLabsApiKeySettings from "@/features/voice/components/ElevenLabsApiKeySettings";
import ElevenLabsModelSelect from "@/features/voice/components/ElevenLabsModelSelect";
import ElevenLabsVoicePicker from "@/features/voice/components/ElevenLabsVoicePicker";
import { SYNTHESIS_MODE_OPTIONS } from "@/features/voice/lib/voiceSettings";
import type { ToastType } from "@/shared/context/toastTypes";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";
import type { ConfigView, ElevenLabsModelOption, ElevenLabsVoiceOption, SaveConfigPayload, SaveConfigResult, TtsSynthesisMode } from "@/shared/lib/types/pipeline";

function SelectOptionWithHint({
  value,
  label,
  hint,
}: {
  value: string;
  label: string;
  hint: string;
}) {
  return (
    <SelectItem value={value} textValue={label}>
      <span className="flex items-center gap-1.5">
        <span>{label}</span>
        <SettingInfoHint label={label}>{hint}</SettingInfoHint>
      </span>
    </SelectItem>
  );
}

export interface ElevenLabsCustomVoicePanelProps {
  config: ConfigView;
  direction: "outbound" | "inbound";
  locked: boolean;
  apiKeyLocked?: boolean;
  showApiKey?: boolean;
  customVoiceSectionRef: RefObject<HTMLDivElement | null>;
  voicesNonce: number;
  ttsModel: string;
  setTtsModel: (v: string) => void;
  stability: number;
  setStability: (v: number) => void;
  similarityBoost: number;
  setSimilarityBoost: (v: number) => void;
  synthesisMode: TtsSynthesisMode;
  setSynthesisMode: (v: TtsSynthesisMode) => void;
  customVoiceSettingsDirty: boolean;
  customVoiceSettingsSaving: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTestElevenLabs: (apiKey: string) => Promise<void>;
  onListElevenLabsVoices: (apiKey?: string) => Promise<ElevenLabsVoiceOption[]>;
  onListElevenLabsModels: (apiKey?: string) => Promise<ElevenLabsModelOption[]>;
  onValidateElevenLabsVoice?: (voiceId: string, apiKey?: string) => Promise<void>;
  onPreviewElevenLabsVoice?: (voiceId: string, apiKey?: string) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  onKeyDirty: (dirty: boolean) => void;
  onVoiceDirty: (dirty: boolean) => void;
  onKeySaved: () => void;
  persistElevenLabsVoices: (list: ElevenLabsVoiceOption[]) => Promise<void>;
  persistElevenLabsModels: (list: ElevenLabsModelOption[]) => Promise<void>;
  resetCustomVoiceSettings: () => void;
  persistCustomVoiceSettings: () => void;
}

export default function ElevenLabsCustomVoicePanel({
  config,
  direction,
  locked,
  apiKeyLocked = locked,
  showApiKey = true,
  customVoiceSectionRef,
  voicesNonce,
  ttsModel,
  setTtsModel,
  stability,
  setStability,
  similarityBoost,
  setSimilarityBoost,
  synthesisMode,
  setSynthesisMode,
  customVoiceSettingsDirty,
  customVoiceSettingsSaving,
  onSave,
  onTestElevenLabs,
  onListElevenLabsVoices,
  onListElevenLabsModels,
  onValidateElevenLabsVoice,
  onPreviewElevenLabsVoice,
  onToast,
  onKeyDirty,
  onVoiceDirty,
  onKeySaved,
  persistElevenLabsVoices,
  persistElevenLabsModels,
  resetCustomVoiceSettings,
  persistCustomVoiceSettings,
}: ElevenLabsCustomVoicePanelProps) {
  const speakingStyleAvailable = config.aiProvider !== "soniox";
  const idPrefix = `${direction}-el`;
  const keyConfigured = config.elevenlabsApiKeyConfigured;
  const panelId = `${direction}-elevenlabs-api-key-panel`;
  const [apiKeyPanelOpen, setApiKeyPanelOpen] = useState(false);
  const [keyDirty, setKeyDirty] = useState(false);
  const apiKeyWasDirty = useRef(false);

  useEffect(() => {
    if (!keyConfigured) {
      setApiKeyPanelOpen(false);
      setKeyDirty(false);
      apiKeyWasDirty.current = false;
    }
  }, [keyConfigured]);

  const handleKeyDirty = (dirty: boolean) => {
    setKeyDirty(dirty);
    onKeyDirty(dirty);
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

  const showApiKeyField = showApiKey && (!keyConfigured || apiKeyPanelOpen);
  const chipStatus = keyDirty
    ? { tone: "warn" as const, label: "Not saved" }
    : { tone: "ok" as const, label: "Ready" };

  return (
    <div ref={customVoiceSectionRef} className="flex flex-col gap-4">
      <SettingsGroup
        title="ElevenLabs"
        headerEnd={
          showApiKey && keyConfigured ? (
            <ApiKeyChip
              open={apiKeyPanelOpen}
              status={chipStatus}
              panelId={panelId}
              onToggle={() => setApiKeyPanelOpen((open) => !open)}
            />
          ) : null
        }
      >
        {showApiKeyField ? (
          <div
            id={panelId}
            className={
              keyConfigured
                ? "flex flex-col gap-3 pt-3"
                : undefined
            }
          >
            <ElevenLabsApiKeySettings
              config={config}
              locked={apiKeyLocked}
              onSave={onSave}
              onTest={onTestElevenLabs}
              onToast={onToast}
              onStateChange={({ dirty: nextDirty }) => handleKeyDirty(nextDirty)}
              onKeySaved={onKeySaved}
            />
          </div>
        ) : null}
        <ElevenLabsVoicePicker
          config={config}
          voiceIdField={direction}
          locked={locked}
          apiKeyReady={config.elevenlabsApiKeyConfigured}
          voicesNonce={voicesNonce}
          cachedVoices={config.elevenlabsVoices ?? []}
          onPersistVoices={persistElevenLabsVoices}
          onSave={onSave}
          onListVoices={onListElevenLabsVoices}
          onValidate={onValidateElevenLabsVoice}
          onPreview={onPreviewElevenLabsVoice}
          onToast={onToast}
          onStateChange={({ dirty: nextDirty }) => onVoiceDirty(nextDirty)}
        />
      </SettingsGroup>

      <Collapsible className="group flex flex-col gap-3">
        <div className="flex w-full items-center gap-1.5">
          <CollapsibleTrigger asChild>
            <button
              type="button"
              className="flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-2 rounded-md text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <SectionHeading as="h4">Advanced</SectionHeading>
              <ChevronDown className="size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
            </button>
          </CollapsibleTrigger>
          <SettingInfoHint label="About advanced voice settings">
            {speakingStyleAvailable
              ? "Speaking style, TTS model, stability, and similarity. Defaults work for most meetings — open only if custom voice quality needs tuning."
              : "TTS model, stability, and similarity. Speaking style is managed by the Soniox live pipeline. Defaults work for most meetings."}
          </SettingInfoHint>
        </div>
        <CollapsibleContent className="flex flex-col gap-4">
          {speakingStyleAvailable ? (
            <div className="space-y-2">
              <span className="inline-flex items-center gap-1.5">
                <Label
                  htmlFor={`${idPrefix}-synthesis-mode`}
                  className={settingsFieldLabelClass}
                >
                  Speaking style
                </Label>
                <SettingInfoHint label="About speaking style">
                  Main control for speech smoothness vs latency. Speed flushes at
                  each sentence end; Natural waits for full sentences.
                </SettingInfoHint>
              </span>
              <Select
                value={synthesisMode}
                onValueChange={(value) =>
                  setSynthesisMode(value as TtsSynthesisMode)
                }
                disabled={locked}
              >
                <SelectTrigger
                  id={`${idPrefix}-synthesis-mode`}
                  className="w-full"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SYNTHESIS_MODE_OPTIONS.map((option) => (
                    <SelectOptionWithHint
                      key={option.value}
                      value={option.value}
                      label={option.label}
                      hint={option.hint}
                    />
                  ))}
                </SelectContent>
              </Select>
            </div>
          ) : null}

          <ElevenLabsModelSelect
            value={ttsModel}
            onValueChange={setTtsModel}
            disabled={locked}
            apiKeyReady={config.elevenlabsApiKeyConfigured}
            refreshNonce={voicesNonce}
            cachedModels={config.elevenlabsModels ?? []}
            onPersistModels={persistElevenLabsModels}
            onListModels={onListElevenLabsModels}
          />

          <div className="space-y-2">
            <span className="inline-flex items-center gap-1.5">
              <Label
                htmlFor={`${idPrefix}-stability`}
                className={settingsFieldLabelClass}
              >
                Stability ({stability.toFixed(2)})
              </Label>
              <SettingInfoHint label="About stability">
                Higher = steadier delivery; lower = more expressive variation.
              </SettingInfoHint>
            </span>
            <input
              id={`${idPrefix}-stability`}
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={stability}
              disabled={locked}
              aria-valuetext={stability.toFixed(2)}
              onChange={(e) => setStability(Number(e.target.value))}
              className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
              style={rangeTrackStyle(stability, 0, 1)}
            />
          </div>

          <div className="space-y-2">
            <span className="inline-flex items-center gap-1.5">
              <Label
                htmlFor={`${idPrefix}-similarity`}
                className={settingsFieldLabelClass}
              >
                Similarity boost ({similarityBoost.toFixed(2)})
              </Label>
              <SettingInfoHint label="About similarity boost">
                Higher = closer to your voice sample; lower = more model
                interpretation.
              </SettingInfoHint>
            </span>
            <input
              id={`${idPrefix}-similarity`}
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={similarityBoost}
              disabled={locked}
              aria-valuetext={similarityBoost.toFixed(2)}
              onChange={(e) => setSimilarityBoost(Number(e.target.value))}
              className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
              style={rangeTrackStyle(similarityBoost, 0, 1)}
            />
          </div>
        </CollapsibleContent>
      </Collapsible>

      {customVoiceSettingsDirty ? (
        <div className="flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={locked || customVoiceSettingsSaving}
            onClick={resetCustomVoiceSettings}
          >
            Cancel
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={locked || customVoiceSettingsSaving}
            onClick={persistCustomVoiceSettings}
          >
            {customVoiceSettingsSaving ? "Saving…" : "Save voice settings"}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
