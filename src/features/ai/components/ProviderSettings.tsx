import { useCallback, useState } from "react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";

import { migrateConfigForProvider } from "../lib/aiApi";
import type { AiProvider } from "../lib/aiTypes";
import { providerLabel } from "../lib/aiTypes";
import { invalidateAiCatalog } from "../hooks/useAiCatalog";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";

interface Props {
  config: ConfigView;
  locked: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
}

const PROVIDERS: AiProvider[] = ["gemini", "openAi", "soniox"];

export default function ProviderSettings({
  config,
  locked,
  onSave,
  onToast,
}: Props) {
  const [saving, setSaving] = useState(false);
  const notesMode = config.sessionMode === "notes";
  const availableProviders = notesMode
    ? PROVIDERS.filter((p) => p !== "gemini")
    : PROVIDERS;

  const handleProviderChange = useCallback(
    async (nextProvider: AiProvider) => {
      if (nextProvider === config.aiProvider || locked) return;
      if (notesMode && nextProvider === "gemini") {
        onToast("error", "Notes mode does not support Gemini. Use OpenAI or Soniox.");
        return;
      }
      setSaving(true);
      try {
        invalidateAiCatalog(nextProvider);
        await onSave(toSavePayload({ ...config, aiProvider: nextProvider }));
        const migrated = await migrateConfigForProvider(nextProvider);
        const changed =
          migrated.myLanguage !== config.myLanguage ||
          migrated.meetingLanguage !== config.meetingLanguage ||
          migrated.liveModel !== config.liveModel ||
          migrated.summaryModel !== config.summaryModel;
        await onSave(toSavePayload(migrated));
        onToast(
          "success",
          changed
            ? `Switched to ${providerLabel(nextProvider)} — settings adjusted`
            : `${providerLabel(nextProvider)} provider saved`,
        );
      } catch (e) {
        onToast("error", String(e));
      } finally {
        setSaving(false);
      }
    },
    [config, locked, notesMode, onSave, onToast],
  );

  return (
    <div className="flex flex-col gap-2">
      <Select
        value={
          availableProviders.includes(config.aiProvider)
            ? config.aiProvider
            : (availableProviders[0] ?? "soniox")
        }
        disabled={locked || saving}
        onValueChange={(value) => void handleProviderChange(value as AiProvider)}
      >
        <SelectTrigger
          id="settings-ai-provider"
          className="w-full"
          aria-label="Engine"
        >
          <SelectValue placeholder="Select engine" />
        </SelectTrigger>
        <SelectContent position="popper">
          {availableProviders.map((provider) => (
            <SelectItem key={provider} value={provider}>
              {providerLabel(provider)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {notesMode ? (
        <p className="m-0 text-xs text-muted-foreground">
          Notes mode supports OpenAI and Soniox only (STT-only). Gemini is
          hidden.
        </p>
      ) : null}
      {locked ? (
        <p className="m-0 text-sm text-muted-foreground">
          {notesMode
            ? "Stop notes to change engine"
            : "Stop translation to change engine"}
        </p>
      ) : null}
    </div>
  );
}
