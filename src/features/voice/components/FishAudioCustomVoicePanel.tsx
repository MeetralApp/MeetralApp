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
import FishAudioApiKeySettings from "@/features/voice/components/FishAudioApiKeySettings";
import FishAudioModelSelect from "@/features/voice/components/FishAudioModelSelect";
import FishAudioVoicePicker from "@/features/voice/components/FishAudioVoicePicker";
import { FISH_LATENCY_OPTIONS } from "@/features/voice/lib/voiceSettings";
import type { ToastType } from "@/shared/context/toastTypes";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";
import type {
  ConfigView,
  FishAudioLatency,
  FishAudioModelOption,
  FishAudioVoiceOption,
  SaveConfigPayload,
  SaveConfigResult,
} from "@/shared/lib/types/pipeline";

export interface FishAudioCustomVoicePanelProps {
  config: ConfigView;
  direction: "outbound" | "inbound";
  locked: boolean;
  apiKeyLocked?: boolean;
  showApiKey?: boolean;
  customVoiceSectionRef: RefObject<HTMLDivElement | null>;
  voicesNonce: number;
  ttsModel: string;
  setTtsModel: (v: string) => void;
  latency: FishAudioLatency;
  setLatency: (v: FishAudioLatency) => void;
  temperature: number;
  setTemperature: (v: number) => void;
  speed: number;
  setSpeed: (v: number) => void;
  topP: number;
  setTopP: (v: number) => void;
  customVoiceSettingsDirty: boolean;
  customVoiceSettingsSaving: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTestFishAudio: (apiKey: string) => Promise<void>;
  onListFishAudioVoices: (apiKey?: string) => Promise<FishAudioVoiceOption[]>;
  onListFishAudioModels: () => Promise<FishAudioModelOption[]>;
  onValidateFishAudioVoice?: (voiceId: string, apiKey?: string) => Promise<void>;
  onPreviewFishAudioVoice?: (voiceId: string, apiKey?: string) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  onKeyDirty: (dirty: boolean) => void;
  onVoiceDirty: (dirty: boolean) => void;
  onKeySaved: () => void;
  persistFishAudioVoices: (list: FishAudioVoiceOption[]) => Promise<void>;
  persistFishAudioModels: (list: FishAudioModelOption[]) => Promise<void>;
  resetCustomVoiceSettings: () => void;
  persistCustomVoiceSettings: () => void;
}

export default function FishAudioCustomVoicePanel({
  config,
  direction,
  locked,
  apiKeyLocked = locked,
  showApiKey = true,
  customVoiceSectionRef,
  voicesNonce,
  ttsModel,
  setTtsModel,
  latency,
  setLatency,
  temperature,
  setTemperature,
  speed,
  setSpeed,
  topP,
  setTopP,
  customVoiceSettingsDirty,
  customVoiceSettingsSaving,
  onSave,
  onTestFishAudio,
  onListFishAudioVoices,
  onListFishAudioModels,
  onValidateFishAudioVoice,
  onPreviewFishAudioVoice,
  onToast,
  onKeyDirty,
  onVoiceDirty,
  onKeySaved,
  persistFishAudioVoices,
  persistFishAudioModels,
  resetCustomVoiceSettings,
  persistCustomVoiceSettings,
}: FishAudioCustomVoicePanelProps) {
  const idPrefix = `${direction}-fish`;
  const keyConfigured = Boolean(config.fishaudioApiKeyConfigured);
  const panelId = `${direction}-fishaudio-api-key-panel`;
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
        title="Fish Audio"
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
            className={keyConfigured ? "flex flex-col gap-3 pt-3" : undefined}
          >
            <FishAudioApiKeySettings
              config={config}
              locked={apiKeyLocked}
              onSave={onSave}
              onTest={onTestFishAudio}
              onToast={onToast}
              onStateChange={({ dirty: nextDirty }) => handleKeyDirty(nextDirty)}
              onKeySaved={onKeySaved}
            />
          </div>
        ) : null}
        <FishAudioVoicePicker
          config={config}
          voiceIdField={direction}
          locked={locked}
          apiKeyReady={keyConfigured}
          voicesNonce={voicesNonce}
          cachedVoices={config.fishaudioVoices ?? []}
          onPersistVoices={persistFishAudioVoices}
          onSave={onSave}
          onListVoices={onListFishAudioVoices}
          onValidate={onValidateFishAudioVoice}
          onPreview={onPreviewFishAudioVoice}
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
          <SettingInfoHint label="About Fish Audio advanced settings">
            Model, latency, temperature, speed, and top-p for this direction.
            Defaults work for most meetings. s2.1-pro-free has no latency SLA.
          </SettingInfoHint>
        </div>
        <CollapsibleContent className="flex flex-col gap-4">
          <FishAudioModelSelect
            value={ttsModel}
            onValueChange={setTtsModel}
            disabled={locked}
            apiKeyReady={keyConfigured}
            refreshNonce={voicesNonce}
            cachedModels={config.fishaudioModels ?? []}
            onPersistModels={persistFishAudioModels}
            onListModels={onListFishAudioModels}
          />

          <div className="space-y-2">
            <Label htmlFor={`${idPrefix}-latency`} className={settingsFieldLabelClass}>
              Latency
            </Label>
            <Select
              value={latency}
              onValueChange={(value) => setLatency(value as FishAudioLatency)}
              disabled={locked}
            >
              <SelectTrigger id={`${idPrefix}-latency`} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {FISH_LATENCY_OPTIONS.map((option) => (
                  <SelectItem
                    key={option.value}
                    value={option.value}
                    textValue={option.label}
                  >
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label
              htmlFor={`${idPrefix}-temperature`}
              className={settingsFieldLabelClass}
            >
              Temperature ({temperature.toFixed(2)})
            </Label>
            <input
              id={`${idPrefix}-temperature`}
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={temperature}
              disabled={locked}
              aria-valuetext={temperature.toFixed(2)}
              onChange={(e) => setTemperature(Number(e.target.value))}
              className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
              style={rangeTrackStyle(temperature, 0, 1)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor={`${idPrefix}-speed`} className={settingsFieldLabelClass}>
              Speed ({speed.toFixed(2)})
            </Label>
            <input
              id={`${idPrefix}-speed`}
              type="range"
              min={0.5}
              max={2}
              step={0.05}
              value={speed}
              disabled={locked}
              aria-valuetext={speed.toFixed(2)}
              onChange={(e) => setSpeed(Number(e.target.value))}
              className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
              style={rangeTrackStyle(speed, 0.5, 2)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor={`${idPrefix}-top-p`} className={settingsFieldLabelClass}>
              Top-p ({topP.toFixed(2)})
            </Label>
            <input
              id={`${idPrefix}-top-p`}
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={topP}
              disabled={locked}
              aria-valuetext={topP.toFixed(2)}
              onChange={(e) => setTopP(Number(e.target.value))}
              className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
              style={rangeTrackStyle(topP, 0, 1)}
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
