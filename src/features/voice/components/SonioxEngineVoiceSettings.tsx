import { useEffect, useState } from "react";
import { Play, RefreshCw } from "lucide-react";

import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SettingsField from "@/shared/components/SettingsField";
import SettingsGroup from "@/shared/components/SettingsGroup";
import SettingsIconButton from "@/shared/components/SettingsIconButton";
import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import type { SonioxTtsModelOption } from "@/features/ai/lib/aiTypes";
import type { ConfigView, SaveConfigResult, SonioxVoiceOption } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";

export type SonioxEngineDirection = "inbound" | "outbound";

type SonioxVoicePatch = Parameters<typeof toSavePayload>[1];

interface Props {
  config: ConfigView;
  direction: SonioxEngineDirection;
  locked: boolean;
  ttsModels: SonioxTtsModelOption[];
  sonioxVoices: SonioxVoiceOption[];
  catalogLoading: boolean;
  /** Shared TTS model — same field on both Engine columns. */
  showSharedModel?: boolean;
  onRefreshCatalog: () => void;
  onPreviewVoice?: (voice: string, apiKey?: string) => Promise<void>;
  onSave: (patch: SonioxVoicePatch) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
}

export default function SonioxEngineVoiceSettings({
  config,
  direction,
  locked,
  ttsModels,
  sonioxVoices,
  catalogLoading,
  showSharedModel = true,
  onRefreshCatalog,
  onPreviewVoice,
  onSave,
  onToast,
}: Props) {
  const isInbound = direction === "inbound";
  const directionLabel = isInbound ? "Meeting → You" : "You → Meeting";
  const selectedModel = config.sonioxTtsModel ?? "tts-rt-v1";
  const [previewing, setPreviewing] = useState(false);
  const [draftModel, setDraftModel] = useState(selectedModel);
  useEffect(() => {
    setDraftModel(selectedModel);
  }, [selectedModel]);
  const selectedVoice = isInbound
    ? (config.sonioxTtsVoice ?? "Adrian")
    : (config.sonioxTtsOutboundVoice ??
      config.sonioxTtsVoice ??
      "Adrian");
  const speed = isInbound
    ? (config.sonioxTtsInboundSpeed ?? 1)
    : (config.sonioxTtsOutboundSpeed ?? 1);
  const modelId = isInbound
    ? "soniox-tts-inbound-model"
    : "soniox-tts-outbound-model";
  const voiceId = isInbound
    ? "soniox-tts-inbound-voice"
    : "soniox-tts-outbound-voice";
  const speedId = isInbound
    ? "soniox-tts-inbound-speed"
    : "soniox-tts-outbound-speed";

  const handlePreview = async () => {
    if (!onPreviewVoice || !selectedVoice.trim()) return;
    setPreviewing(true);
    try {
      await onPreviewVoice(selectedVoice.trim());
      onToast("success", "Playing voice preview on local playback");
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setPreviewing(false);
    }
  };

  const disabled = locked || catalogLoading;

  const refreshAction = (
    <SettingsIconButton
      label={
        !config.sonioxApiKeyConfigured
          ? "Add a Soniox API key to refresh"
          : catalogLoading
            ? "Refreshing…"
            : "Refresh voices"
      }
      icon={RefreshCw}
      disabled={locked || catalogLoading || !config.sonioxApiKeyConfigured}
      busy={catalogLoading}
      onClick={onRefreshCatalog}
    />
  );

  const previewAction = onPreviewVoice ? (
    <SettingsIconButton
      label={
        locked
          ? `Stop ${isInbound ? "inbound" : "outbound"} to preview voice`
          : !config.sonioxApiKeyConfigured
            ? "Add a Soniox API key to preview"
            : previewing
              ? "Playing…"
              : "Preview voice"
      }
      icon={Play}
      disabled={
        locked ||
        previewing ||
        catalogLoading ||
        !config.sonioxApiKeyConfigured ||
        !selectedVoice.trim()
      }
      onClick={() => void handlePreview()}
    />
  ) : null;

  return (
    <SettingsGroup
      title="Soniox TTS"
      titleHint={
        <SettingInfoHint label={`About ${directionLabel} Soniox TTS`}>
          Voice and speed are per-direction. TTS model is shared across both
          directions. Language follows Translate → Languages.
        </SettingInfoHint>
      }
      headerEnd={refreshAction}
    >
      {showSharedModel ? (
        <SettingsField label="TTS model" htmlFor={modelId}>
          <Select
            value={draftModel}
            disabled={disabled || ttsModels.length === 0}
            onValueChange={(model) => {
              setDraftModel(model);
              void onSave({ sonioxTtsModel: model })
                .then(() => {
                  onToast("success", "Soniox TTS model saved");
                })
                .catch((e) => {
                  setDraftModel(selectedModel);
                  onToast("error", String(e));
                });
            }}
          >
            <SelectTrigger id={modelId} className="w-full">
              <SelectValue
                placeholder={catalogLoading ? "Loading…" : "Select model"}
              />
            </SelectTrigger>
            <SelectContent>
              {ttsModels.map((model) => (
                <SelectItem key={model.id} value={model.id}>
                  {model.name ?? model.id}
                </SelectItem>
              ))}
              {draftModel &&
              !ttsModels.some((m) => m.id === draftModel) ? (
                <SelectItem value={draftModel}>
                  {draftModel} (saved)
                </SelectItem>
              ) : null}
            </SelectContent>
          </Select>
        </SettingsField>
      ) : null}

      <SettingsField
        label="TTS voice"
        htmlFor={voiceId}
        actions={previewAction}
      >
        <Select
          value={selectedVoice}
          disabled={disabled}
          onValueChange={(voice) => {
            const payload = isInbound
              ? { sonioxTtsVoice: voice }
              : { sonioxTtsOutboundVoice: voice };
            void onSave(payload)
              .then(() =>
                onToast(
                  "success",
                  `${directionLabel} Soniox TTS voice saved`,
                ),
              )
              .catch((e) => onToast("error", String(e)));
          }}
        >
          <SelectTrigger id={voiceId} className="w-full">
            <SelectValue
              placeholder={
                catalogLoading ? "Loading voices…" : "Select voice"
              }
            />
          </SelectTrigger>
          <SelectContent>
            {sonioxVoices.map((voice) => (
              <SelectItem key={voice.id} value={voice.id}>
                {voice.name}
                {voice.gender
                  ? ` (${voice.gender.charAt(0).toUpperCase()}${voice.gender.slice(1)})`
                  : ""}
              </SelectItem>
            ))}
            {selectedVoice &&
            !sonioxVoices.some((v) => v.id === selectedVoice) ? (
              <SelectItem value={selectedVoice}>
                {selectedVoice} (saved)
              </SelectItem>
            ) : null}
          </SelectContent>
        </Select>
      </SettingsField>

      <SettingsField
        labelContent={
          <span className="inline-flex items-center gap-1.5">
            <Label htmlFor={speedId} className={settingsFieldLabelClass}>
              Speed ({speed.toFixed(2)})
            </Label>
            <SettingInfoHint label={`About ${directionLabel} speed`}>
              Speaking rate for {directionLabel}. Range 0.7–1.3; 1.0 is normal.
            </SettingInfoHint>
          </span>
        }
      >
        <input
          id={speedId}
          type="range"
          min={0.7}
          max={1.3}
          step={0.05}
          value={speed}
          disabled={disabled}
          aria-valuetext={speed.toFixed(2)}
          onChange={(e) => {
            const next = Number(e.target.value);
            const payload = isInbound
              ? { sonioxTtsInboundSpeed: next }
              : { sonioxTtsOutboundSpeed: next };
            void onSave(payload)
              .then(() =>
                onToast("success", `${directionLabel} speed saved`),
              )
              .catch((err) => onToast("error", String(err)));
          }}
          className="h-2 w-full cursor-pointer disabled:cursor-not-allowed"
          style={rangeTrackStyle(speed, 0.7, 1.3)}
        />
      </SettingsField>

      {!config.sonioxApiKeyConfigured ? (
        <p className="m-0 text-xs text-muted-foreground">
          Add a Soniox API key to load the full TTS catalog.
        </p>
      ) : null}
      {locked ? (
        <p className="m-0 text-xs text-muted-foreground">
          Stop {isInbound ? "inbound" : "outbound"} translate to change{" "}
          {directionLabel} voice settings.
        </p>
      ) : null}
    </SettingsGroup>
  );
}
