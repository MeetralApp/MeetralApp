import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Play, RefreshCw } from "lucide-react";

import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SettingsField from "@/shared/components/SettingsField";
import SettingsIconButton from "@/shared/components/SettingsIconButton";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";

import type { ConfigView, ElevenLabsVoiceOption, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";

export interface ElevenLabsVoicePickerState {
  dirty: boolean;
}

interface Props {
  config: ConfigView;
  voiceIdField?: "outbound" | "inbound";
  locked?: boolean;
  apiKeyReady: boolean;
  voicesNonce: number;
  cachedVoices: ElevenLabsVoiceOption[];
  onPersistVoices: (list: ElevenLabsVoiceOption[]) => Promise<void>;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onListVoices: (apiKey?: string) => Promise<ElevenLabsVoiceOption[]>;
  onValidate?: (voiceId: string, apiKey?: string) => Promise<void>;
  onPreview?: (voiceId: string, apiKey?: string) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  onStateChange?: (state: ElevenLabsVoicePickerState) => void;
}

function formatCategory(category: string): string {
  switch (category.toLowerCase()) {
    case "professional":
      return "PVC";
    case "cloned":
      return "Clone";
    case "premade":
      return "Premade";
    case "generated":
      return "Generated";
    default:
      return category;
  }
}

function truncateVoiceId(id: string): string {
  if (id.length <= 14) return id;
  return `${id.slice(0, 6)}…${id.slice(-4)}`;
}

export default function ElevenLabsVoicePicker({
  config,
  voiceIdField = "outbound",
  locked = false,
  apiKeyReady,
  voicesNonce,
  cachedVoices,
  onPersistVoices,
  onSave,
  onListVoices,
  onValidate,
  onPreview,
  onToast,
  onStateChange,
}: Props) {
  const selectId = useId();
  const savedVoiceId =
    voiceIdField === "inbound"
      ? (config.elevenlabsInboundVoiceId ?? "")
      : (config.elevenlabsVoiceId ?? "");
  const directionLabel =
    voiceIdField === "inbound" ? "inbound" : "outbound";
  const lastNonceRef = useRef(voicesNonce);

  const [voices, setVoices] = useState<ElevenLabsVoiceOption[]>(cachedVoices);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedVoiceId, setSelectedVoiceId] = useState(savedVoiceId);
  const [saving, setSaving] = useState(false);
  const [previewing, setPreviewing] = useState(false);

  useEffect(() => {
    setSelectedVoiceId(savedVoiceId);
  }, [savedVoiceId]);

  useEffect(() => {
    // Manual Voice ID UI removed — picker is never draft-dirty on its own.
    onStateChange?.({ dirty: false });
  }, [onStateChange]);

  const refreshVoices = useCallback(async () => {
    if (!apiKeyReady) return;
    setLoading(true);
    setLoadError(null);
    try {
      const list = await onListVoices();
      setVoices(list);
      await onPersistVoices(list);
      if (list.length === 0) {
        setLoadError(
          "No voices in My Voices. Clone a voice or add one from the ElevenLabs Voice Library, then refresh.",
        );
      }
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      setLoadError(message);
      setVoices([]);
    } finally {
      setLoading(false);
    }
  }, [apiKeyReady, onListVoices, onPersistVoices]);

  useEffect(() => {
    if (!apiKeyReady) {
      setVoices([]);
      setLoadError(null);
      lastNonceRef.current = voicesNonce;
      return;
    }
    const nonceChanged = lastNonceRef.current !== voicesNonce;
    lastNonceRef.current = voicesNonce;
    if (!nonceChanged && cachedVoices.length > 0) {
      setVoices(cachedVoices);
      return;
    }
    void refreshVoices();
  }, [apiKeyReady, voicesNonce, cachedVoices, refreshVoices]);

  const persistVoice = useCallback(
    async (voiceId: string) => {
      const trimmed = voiceId.trim();
      if (!trimmed) return;
      setSaving(true);
      try {
        if (onValidate) {
          await onValidate(trimmed);
        }
        await onSave(
          toSavePayload(
            config,
            voiceIdField === "inbound"
              ? { elevenlabsInboundVoiceId: trimmed }
              : { elevenlabsVoiceId: trimmed },
          ),
        );
        setSelectedVoiceId(trimmed);
        onToast("success", "Voice saved");
      } catch (e) {
        onToast(
          "error",
          e instanceof Error ? e.message : "Failed to save voice",
        );
      } finally {
        setSaving(false);
      }
    },
    [config, onSave, onToast, onValidate, voiceIdField],
  );

  const handlePreview = async () => {
    if (!selectedVoiceId.trim() || !onPreview) return;
    setPreviewing(true);
    try {
      if (onValidate) {
        await onValidate(selectedVoiceId.trim());
      }
      await onPreview(selectedVoiceId.trim());
      onToast("success", "Playing voice preview on local playback");
    } catch (e) {
      onToast("error", e instanceof Error ? e.message : "Preview failed");
    } finally {
      setPreviewing(false);
    }
  };

  const handleSelectChange = (value: string) => {
    setSelectedVoiceId(value);
    if (value.trim() && value.trim() !== savedVoiceId.trim()) {
      void persistVoice(value);
    }
  };

  if (!apiKeyReady) {
    return (
      <p className="text-muted-foreground text-sm">
        Save your ElevenLabs API key to load available voices.
      </p>
    );
  }

  return (
    <SettingsField
      labelContent={
        <span className="inline-flex items-center gap-1.5">
          <Label htmlFor={selectId} className={settingsFieldLabelClass}>
            Clone voice
          </Label>
          <SettingInfoHint label="About cloned voice">
            Lists voices from your ElevenLabs My Voices. Professional Voice
            Clones work best for meeting translation.
          </SettingInfoHint>
        </span>
      }
      actions={
        <>
          {onPreview ? (
            <SettingsIconButton
              label={
                locked
                  ? `Stop ${directionLabel} to preview voice`
                  : previewing
                    ? "Playing…"
                    : "Preview voice"
              }
              icon={Play}
              disabled={
                locked || previewing || loading || !selectedVoiceId.trim()
              }
              onClick={() => void handlePreview()}
            />
          ) : null}
          <SettingsIconButton
            label={
              locked
                ? `Stop ${directionLabel} to refresh voices`
                : loading
                  ? "Refreshing…"
                  : "Refresh voices"
            }
            icon={RefreshCw}
            disabled={locked || loading}
            busy={loading}
            onClick={() => void refreshVoices()}
          />
        </>
      }
    >
      <Select
        value={selectedVoiceId || undefined}
        onValueChange={handleSelectChange}
        disabled={locked || loading || saving || voices.length === 0}
      >
        <SelectTrigger id={selectId} className="w-full">
          <SelectValue
            placeholder={loading ? "Loading voices…" : "Select a voice"}
          />
        </SelectTrigger>
        <SelectContent>
          {selectedVoiceId &&
            !voices.some((v) => v.voiceId === selectedVoiceId) &&
            selectedVoiceId.trim() && (
              <SelectItem
                value={selectedVoiceId}
                textValue={`Unknown voice ${selectedVoiceId}`}
              >
                <span className="flex min-w-0 items-center gap-2">
                  <span className="truncate">Unknown voice</span>
                  <span className="shrink-0 font-mono text-muted-foreground text-xs">
                    ID {truncateVoiceId(selectedVoiceId)}
                  </span>
                </span>
              </SelectItem>
            )}
          {voices.map((voice) => (
            <SelectItem
              key={voice.voiceId}
              value={voice.voiceId}
              textValue={`${voice.name} ${formatCategory(voice.category)} ${voice.voiceId}`}
            >
              <span className="flex min-w-0 items-center gap-2">
                <span className="truncate">{voice.name}</span>
                <span className="shrink-0 text-muted-foreground text-xs">
                  {formatCategory(voice.category)} · ID{" "}
                  <span className="font-mono">
                    {truncateVoiceId(voice.voiceId)}
                  </span>
                </span>
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {loadError ? (
        <p className="text-destructive text-sm" role="alert">
          {loadError}
        </p>
      ) : null}
    </SettingsField>
  );
}
