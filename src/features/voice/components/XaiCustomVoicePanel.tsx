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
import XaiApiKeySettings from "@/features/voice/components/XaiApiKeySettings";
import XaiVoicePicker from "@/features/voice/components/XaiVoicePicker";
import { XAI_LATENCY_OPTIONS } from "@/features/voice/lib/voiceSettings";
import type { ToastType } from "@/shared/context/toastTypes";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";
import type {
  ConfigView,
  XaiLatency,
  XaiVoiceOption,
  SaveConfigPayload,
  SaveConfigResult,
} from "@/shared/lib/types/pipeline";

export interface XaiCustomVoicePanelProps {
  config: ConfigView;
  direction: "outbound" | "inbound";
  locked: boolean;
  apiKeyLocked?: boolean;
  showApiKey?: boolean;
  customVoiceSectionRef: RefObject<HTMLDivElement | null>;
  voicesNonce: number;
  latency: XaiLatency;
  setLatency: (v: XaiLatency) => void;
  speed: number;
  setSpeed: (v: number) => void;
  customVoiceSettingsDirty: boolean;
  customVoiceSettingsSaving: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTestXai: (apiKey: string) => Promise<void>;
  onListXaiVoices: (apiKey?: string) => Promise<XaiVoiceOption[]>;
  onValidateXaiVoice?: (voiceId: string, apiKey?: string) => Promise<void>;
  onPreviewXaiVoice?: (voiceId: string, apiKey?: string) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  onKeyDirty: (dirty: boolean) => void;
  onVoiceDirty: (dirty: boolean) => void;
  onKeySaved: () => void;
  persistXaiVoices: (list: XaiVoiceOption[]) => Promise<void>;
  resetCustomVoiceSettings: () => void;
  persistCustomVoiceSettings: () => void;
}

export default function XaiCustomVoicePanel({
  config,
  direction,
  locked,
  apiKeyLocked = locked,
  showApiKey = true,
  customVoiceSectionRef,
  voicesNonce,
  latency,
  setLatency,
  speed,
  setSpeed,
  customVoiceSettingsDirty,
  customVoiceSettingsSaving,
  onSave,
  onTestXai,
  onListXaiVoices,
  onValidateXaiVoice,
  onPreviewXaiVoice,
  onToast,
  onKeyDirty,
  onVoiceDirty,
  onKeySaved,
  persistXaiVoices,
  resetCustomVoiceSettings,
  persistCustomVoiceSettings,
}: XaiCustomVoicePanelProps) {
  const idPrefix = `${direction}-xai`;
  const keyConfigured = Boolean(config.xaiApiKeyConfigured);
  const panelId = `${direction}-xai-api-key-panel`;
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
        title="xAI"
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
            <XaiApiKeySettings
              config={config}
              locked={apiKeyLocked}
              onSave={onSave}
              onTest={onTestXai}
              onToast={onToast}
              onStateChange={({ dirty: nextDirty }) => handleKeyDirty(nextDirty)}
              onKeySaved={onKeySaved}
            />
          </div>
        ) : null}
        <XaiVoicePicker
          config={config}
          voiceIdField={direction}
          locked={locked}
          apiKeyReady={keyConfigured}
          voicesNonce={voicesNonce}
          cachedVoices={config.xaiVoices ?? []}
          onPersistVoices={persistXaiVoices}
          onSave={onSave}
          onListVoices={onListXaiVoices}
          onValidate={onValidateXaiVoice}
          onPreview={onPreviewXaiVoice}
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
          <SettingInfoHint label="About xAI advanced settings">
            Latency and playback speed for this direction. Defaults work for
            most meetings.
          </SettingInfoHint>
        </div>
        <CollapsibleContent className="flex flex-col gap-4">
          <div className="space-y-2">
            <Label htmlFor={`${idPrefix}-latency`} className={settingsFieldLabelClass}>
              Latency
            </Label>
            <Select
              value={latency}
              onValueChange={(value) => setLatency(value as XaiLatency)}
              disabled={locked}
            >
              <SelectTrigger id={`${idPrefix}-latency`} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {XAI_LATENCY_OPTIONS.map((option) => (
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
            <Label htmlFor={`${idPrefix}-speed`} className={settingsFieldLabelClass}>
              Speed ({speed.toFixed(2)})
            </Label>
            <input
              id={`${idPrefix}-speed`}
              type="range"
              min={0.7}
              max={1.5}
              step={0.05}
              value={speed}
              disabled={locked}
              aria-valuetext={speed.toFixed(2)}
              onChange={(e) => setSpeed(Number(e.target.value))}
              className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
              style={rangeTrackStyle(speed, 0.7, 1.5)}
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
