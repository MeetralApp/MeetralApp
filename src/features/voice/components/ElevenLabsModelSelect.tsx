import { useCallback, useEffect, useId, useRef, useState } from "react";
import { RefreshCw, TriangleAlert } from "lucide-react";

import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import AppTooltip from "@/shared/components/AppTooltip";
import SettingsField from "@/shared/components/SettingsField";
import SettingsIconButton from "@/shared/components/SettingsIconButton";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";

import type { ElevenLabsModelOption } from "@/shared/lib/types/pipeline";

function truncateModelId(id: string): string {
  if (id.length <= 18) return id;
  return `${id.slice(0, 8)}…${id.slice(-6)}`;
}

function modelMayFailClone(model: Pick<
  ElevenLabsModelOption,
  "cloneStreamSupported" | "requiresAlphaAccess"
>): boolean {
  return !model.cloneStreamSupported || model.requiresAlphaAccess;
}

function warningMessage(
  model: Pick<
    ElevenLabsModelOption,
    "cloneStreamSupported" | "requiresAlphaAccess"
  >,
): string {
  if (!model.cloneStreamSupported && model.requiresAlphaAccess) {
    return "This model may not work with real-time custom voice (WebSocket) and may require alpha access. Use Eleven Flash v2.5 or Turbo v2.5 for live translation.";
  }
  if (!model.cloneStreamSupported) {
    return "This model may not work with real-time custom voice (WebSocket stream-input). Use Eleven Flash v2.5 or Turbo v2.5 for live translation.";
  }
  return "This model may require alpha access on your ElevenLabs account.";
}

function ModelWarningHint({
  model,
}: {
  model: Pick<
    ElevenLabsModelOption,
    "cloneStreamSupported" | "requiresAlphaAccess"
  >;
}) {
  return (
    <AppTooltip label={warningMessage(model)} side="right">
      <span
        role="img"
        aria-label="Model compatibility warning"
        className="inline-flex shrink-0 text-amber-500"
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
      >
        <TriangleAlert className="size-3.5" aria-hidden />
      </span>
    </AppTooltip>
  );
}

interface Props {
  value: string;
  onValueChange: (modelId: string) => void;
  disabled?: boolean;
  apiKeyReady: boolean;
  refreshNonce: number;
  cachedModels: ElevenLabsModelOption[];
  onPersistModels: (list: ElevenLabsModelOption[]) => Promise<void>;
  onListModels: (apiKey?: string) => Promise<ElevenLabsModelOption[]>;
}

export default function ElevenLabsModelSelect({
  value,
  onValueChange,
  disabled = false,
  apiKeyReady,
  refreshNonce,
  cachedModels,
  onPersistModels,
  onListModels,
}: Props) {
  const selectId = useId();
  const lastNonceRef = useRef(refreshNonce);
  const [models, setModels] = useState<ElevenLabsModelOption[]>(cachedModels);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const unknownSavedModel =
    value.length > 0 && !models.some((model) => model.modelId === value);

  const loadModels = useCallback(async () => {
    if (!apiKeyReady) {
      setModels([]);
      setLoadError(null);
      return;
    }
    setLoading(true);
    setLoadError(null);
    try {
      const list = await onListModels();
      setModels(list);
      await onPersistModels(list);
    } catch (e) {
      setModels([]);
      setLoadError(e instanceof Error ? e.message : "Failed to load models");
    } finally {
      setLoading(false);
    }
  }, [apiKeyReady, onListModels, onPersistModels]);

  useEffect(() => {
    if (!apiKeyReady) {
      setModels([]);
      setLoadError(null);
      lastNonceRef.current = refreshNonce;
      return;
    }
    const nonceChanged = lastNonceRef.current !== refreshNonce;
    lastNonceRef.current = refreshNonce;
    if (!nonceChanged && cachedModels.length > 0) {
      setModels(cachedModels);
      return;
    }
    void loadModels();
  }, [apiKeyReady, refreshNonce, cachedModels, loadModels]);

  return (
    <SettingsField
      labelContent={
        <span className="inline-flex items-center gap-1.5">
          <Label htmlFor={selectId} className={settingsFieldLabelClass}>
            TTS model
          </Label>
          <SettingInfoHint label="About TTS model">
            Model for real-time ElevenLabs voice over WebSocket. Flash or
            Turbo v2.5 recommended — other models may fail in meetings.
          </SettingInfoHint>
        </span>
      }
      actions={
        <SettingsIconButton
          label={loading ? "Refreshing…" : "Refresh models"}
          icon={RefreshCw}
          disabled={disabled || !apiKeyReady || loading}
          busy={loading}
          onClick={() => void loadModels()}
        />
      }
    >
      <Select
        value={value}
        onValueChange={onValueChange}
        disabled={disabled || !apiKeyReady || loading}
      >
        <SelectTrigger id={selectId} className="w-full">
          <SelectValue
            placeholder={
              apiKeyReady ? "Select a TTS model" : "Add API key to load models"
            }
          />
        </SelectTrigger>
        <SelectContent>
          {unknownSavedModel && (
            <SelectItem value={value} textValue={`Unknown model (${value})`}>
              <span className="flex w-full items-center justify-between gap-2">
                <span className="truncate">
                  Unknown model ({truncateModelId(value)})
                </span>
                <ModelWarningHint
                  model={{
                    cloneStreamSupported: false,
                    requiresAlphaAccess: false,
                  }}
                />
              </span>
            </SelectItem>
          )}
          {models.map((model) => (
            <SelectItem
              key={model.modelId}
              value={model.modelId}
              textValue={model.name}
            >
              <span className="flex w-full items-center justify-between gap-2">
                <span className="truncate">{model.name}</span>
                {modelMayFailClone(model) && <ModelWarningHint model={model} />}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {loadError && (
        <p className="text-destructive text-xs leading-relaxed">{loadError}</p>
      )}
      {!loadError && apiKeyReady && models.length === 0 && !loading && (
        <p className="text-muted-foreground text-xs leading-relaxed">
          No TTS models returned. Check your API key or refresh.
        </p>
      )}
    </SettingsField>
  );
}
