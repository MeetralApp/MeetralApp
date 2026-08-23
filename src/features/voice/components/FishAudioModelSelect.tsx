import { useCallback, useEffect, useId, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";

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

import type { FishAudioModelOption } from "@/shared/lib/types/pipeline";

function truncateModelId(id: string): string {
  if (id.length <= 18) return id;
  return `${id.slice(0, 8)}…${id.slice(-6)}`;
}

interface Props {
  value: string;
  onValueChange: (modelId: string) => void;
  disabled?: boolean;
  apiKeyReady: boolean;
  refreshNonce: number;
  cachedModels: FishAudioModelOption[];
  onPersistModels: (list: FishAudioModelOption[]) => Promise<void>;
  onListModels: () => Promise<FishAudioModelOption[]>;
}

export default function FishAudioModelSelect({
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
  const [models, setModels] = useState<FishAudioModelOption[]>(cachedModels);
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
          <SettingInfoHint label="About Fish Audio TTS model">
            Model for real-time Fish Audio voice over WebSocket. S2.1 Pro is
            recommended for live meetings; s2.1-pro-free has no latency SLA.
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
          {unknownSavedModel ? (
            <SelectItem value={value} textValue={`Unknown model (${value})`}>
              <span className="truncate">
                Unknown model ({truncateModelId(value)})
              </span>
            </SelectItem>
          ) : null}
          {models.map((model) => (
            <SelectItem
              key={model.modelId}
              value={model.modelId}
              textValue={`${model.name} ${model.description ?? ""}`}
            >
              <span className="flex min-w-0 flex-col items-start gap-0.5">
                <span className="truncate">{model.name}</span>
                {model.description ? (
                  <span className="text-muted-foreground text-xs leading-snug">
                    {model.description}
                  </span>
                ) : null}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {loadError ? (
        <p className="text-destructive text-xs leading-relaxed">{loadError}</p>
      ) : null}
      {!loadError && apiKeyReady && models.length === 0 && !loading ? (
        <p className="text-muted-foreground text-xs leading-relaxed">
          No TTS models returned. Refresh to reload the allow-list.
        </p>
      ) : null}
    </SettingsField>
  );
}
