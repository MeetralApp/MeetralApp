import { useCallback, useState } from "react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";

import type { AiModelInfo, LiveModelOption } from "@/features/ai/lib/aiTypes";
import { isNotesSession } from "@/features/pipeline/lib/sessionMode";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";

interface Props {
  config: ConfigView;
  models: LiveModelOption[];
  loading?: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
  /** Fixed Notes STT model for OpenAI (read-only; does not write config.liveModel). */
  notesSttModel?: AiModelInfo | null;
}

/** Live translate model — or fixed Notes STT when OpenAI Notes. Summary models on Summaries tab. */
export default function AiModelSettings({
  config,
  models,
  loading = false,
  onSave,
  onToast,
  notesSttModel = null,
}: Props) {
  const [saving, setSaving] = useState(false);
  const notesOpenAiFixed =
    isNotesSession(config) &&
    config.aiProvider === "openAi" &&
    notesSttModel != null;

  const persistLiveModel = useCallback(
    async (modelId: string) => {
      setSaving(true);
      try {
        const selected = models.find((m) => m.id === modelId);
        const langs = selected?.languages ?? [];
        let myLanguage = config.myLanguage;
        let meetingLanguage = config.meetingLanguage;
        if (langs.length > 0) {
          if (!langs.some((l) => l.code === myLanguage)) {
            myLanguage = langs.find((l) => l.code === "vi")?.code ?? langs[0].code;
          }
          if (!langs.some((l) => l.code === meetingLanguage)) {
            meetingLanguage =
              langs.find((l) => l.code === "en")?.code ?? langs[0].code;
          }
        }
        await onSave(
          toSavePayload(
            { ...config, liveModel: modelId, myLanguage, meetingLanguage },
            { liveModel: modelId },
          ),
        );
        onToast("success", "Live translate model saved");
      } catch (e) {
        onToast("error", String(e));
      } finally {
        setSaving(false);
      }
    },
    [config, models, onSave, onToast],
  );

  if (notesOpenAiFixed && notesSttModel) {
    return (
      <div className="flex flex-col gap-1.5">
        <div
          id="settings-live-model"
          role="status"
          aria-label="Notes STT model"
          className="flex h-9 w-full items-center rounded-md border border-border bg-secondary/40 px-3 text-sm text-foreground"
        >
          {notesSttModel.label}
        </div>
        <p className="text-xs text-muted-foreground">
          Notes uses Realtime transcription ({notesSttModel.id}). Interpreter
          keeps GPT Realtime Translate in config when you switch back.
        </p>
      </div>
    );
  }

  const value =
    models.some((m) => m.id === config.liveModel) || models.length === 0
      ? config.liveModel
      : models[0]?.id;

  return (
    <Select
      value={value}
      disabled={saving || loading || models.length === 0}
      onValueChange={(modelId) => void persistLiveModel(modelId)}
    >
      <SelectTrigger
        id="settings-live-model"
        className="w-full"
        aria-label="Live translate model"
      >
        <SelectValue placeholder="Select live translate model" />
      </SelectTrigger>
      <SelectContent position="popper" className="max-h-60">
        {models.map((model) => (
          <SelectItem key={model.id} value={model.id}>
            {model.name ?? model.id}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
