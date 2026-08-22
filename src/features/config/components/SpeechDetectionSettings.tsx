import { useCallback, useEffect, useMemo, useState } from "react";

import { Button } from "@/shared/ui/button";
import { Checkbox } from "@/shared/ui/checkbox";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { cn } from "@/shared/lib/utils";
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

import { useAiCatalog } from "@/features/ai/hooks/useAiCatalog";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";

interface Props {
  config: ConfigView;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
  onDirtyChange?: (dirty: boolean) => void;
}

type MicSensitivity = "low" | "normal" | "high";

type Draft = {
  echoTargetLanguage: boolean;
  vadSilenceDurationMs: number;
  micSensitivity: MicSensitivity;
};

const DEFAULT_ECHO_TARGET_LANGUAGE = true;
const DEFAULT_VAD_SILENCE_MS = 800;
const DEFAULT_MIC_SENSITIVITY: MicSensitivity = "low";

function micSensitivityFromConfig(
  start: ConfigView["vadStartSensitivity"],
  end: ConfigView["vadEndSensitivity"],
): MicSensitivity {
  if (start === "HIGH" || end === "HIGH") return "high";
  if (start === "MEDIUM" || end === "MEDIUM") return "normal";
  return "low";
}

function vadFromMicSensitivity(sensitivity: MicSensitivity): {
  vadStartSensitivity: ConfigView["vadStartSensitivity"];
  vadEndSensitivity: ConfigView["vadEndSensitivity"];
} {
  switch (sensitivity) {
    case "high":
      return { vadStartSensitivity: "HIGH", vadEndSensitivity: "HIGH" };
    case "normal":
      return {
        vadStartSensitivity: "MEDIUM",
        vadEndSensitivity: "MEDIUM",
      };
    default:
      return { vadStartSensitivity: "LOW", vadEndSensitivity: "LOW" };
  }
}

function micSensitivityLabel(sensitivity: MicSensitivity): string {
  switch (sensitivity) {
    case "high":
      return "High";
    case "normal":
      return "Normal";
    default:
      return "Low";
  }
}

function formatOnOff(value: boolean): string {
  return value ? "On" : "Off";
}

function draftFromConfig(config: ConfigView): Draft {
  return {
    echoTargetLanguage: config.echoTargetLanguage,
    vadSilenceDurationMs: config.vadSilenceDurationMs,
    micSensitivity: micSensitivityFromConfig(
      config.vadStartSensitivity,
      config.vadEndSensitivity,
    ),
  };
}

function draftsEqual(a: Draft, b: Draft): boolean {
  return (
    a.echoTargetLanguage === b.echoTargetLanguage &&
    a.vadSilenceDurationMs === b.vadSilenceDurationMs &&
    a.micSensitivity === b.micSensitivity
  );
}

function FieldLabelWithHint({
  name,
  hintName,
  description,
  defaultLabel,
  htmlFor,
}: {
  name: string;
  hintName?: string;
  description: string;
  defaultLabel: string;
  htmlFor?: string;
}) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <Label
        htmlFor={htmlFor}
        className={cn(settingsFieldLabelClass, "cursor-pointer")}
      >
        {name}
      </Label>
      <SettingInfoHint label={`About ${hintName ?? name}`}>
        <p className="m-0">
          {description} Default: {defaultLabel}.
        </p>
      </SettingInfoHint>
    </span>
  );
}

export default function SpeechDetectionSettings({
  config,
  onSave,
  onToast,
  onDirtyChange,
}: Props) {
  const catalog = useAiCatalog(config.aiProvider);
  const supportsGeminiOptions = catalog.capabilities.supportsVadConfig;
  const saved = useMemo(() => draftFromConfig(config), [config]);
  const [draft, setDraft] = useState(saved);
  const [saving, setSaving] = useState(false);

  const dirty = supportsGeminiOptions && !draftsEqual(draft, saved);

  useEffect(() => {
    setDraft(saved);
  }, [saved]);

  useEffect(() => {
    onDirtyChange?.(dirty);
    return () => onDirtyChange?.(false);
  }, [dirty, onDirtyChange]);

  const persistDraft = useCallback(async () => {
    setSaving(true);
    try {
      const vad = vadFromMicSensitivity(draft.micSensitivity);
      await onSave(
        toSavePayload({
          ...config,
          echoTargetLanguage: draft.echoTargetLanguage,
          vadSilenceDurationMs: draft.vadSilenceDurationMs,
          ...vad,
        }),
      );
      onToast("success", "Settings saved");
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setSaving(false);
    }
  }, [config, draft, onSave, onToast]);

  if (!supportsGeminiOptions) return null;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-start gap-3">
        <Checkbox
          id="echo-target-language"
          checked={draft.echoTargetLanguage}
          onCheckedChange={(checked) =>
            setDraft((d) => ({
              ...d,
              echoTargetLanguage: checked === true,
            }))
          }
        />
        <FieldLabelWithHint
          htmlFor="echo-target-language"
          name="Repeat in target language"
          description="Tells Gemini Live to echo translated speech in the target language during outbound translation."
          defaultLabel={formatOnOff(DEFAULT_ECHO_TARGET_LANGUAGE)}
        />
      </div>

      <SettingsField
        labelContent={
          <FieldLabelWithHint
            htmlFor="vad-silence-duration"
            name={`Pause before ending speech (${draft.vadSilenceDurationMs} ms)`}
            hintName="Pause before ending speech"
            description="How long Gemini waits in silence before treating your speech as finished. Lower reacts faster; higher reduces cut-offs."
            defaultLabel={`${DEFAULT_VAD_SILENCE_MS} ms`}
          />
        }
      >
        <input
          id="vad-silence-duration"
          type="range"
          min={100}
          max={2000}
          step={50}
          value={draft.vadSilenceDurationMs}
          aria-valuetext={`${draft.vadSilenceDurationMs} milliseconds`}
          className="h-2 w-full cursor-pointer"
          style={rangeTrackStyle(draft.vadSilenceDurationMs, 100, 2000)}
          onChange={(e) =>
            setDraft((d) => ({
              ...d,
              vadSilenceDurationMs: Number(e.target.value),
            }))
          }
        />
      </SettingsField>

      <SettingsField
        labelContent={
          <FieldLabelWithHint
            name="Mic sensitivity"
            description="Controls voice-activity detection for when Gemini starts and stops listening. Higher picks up quieter or shorter speech."
            defaultLabel={micSensitivityLabel(DEFAULT_MIC_SENSITIVITY)}
          />
        }
      >
        <Select
          value={draft.micSensitivity}
          onValueChange={(next) =>
            setDraft((d) => ({
              ...d,
              micSensitivity: next as MicSensitivity,
            }))
          }
        >
          <SelectTrigger className="w-full" aria-label="Mic sensitivity">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="low">Low</SelectItem>
            <SelectItem value="normal">Normal</SelectItem>
            <SelectItem value="high">High</SelectItem>
          </SelectContent>
        </Select>
      </SettingsField>

      {dirty && (
        <div className="flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={saving}
            onClick={() => setDraft(saved)}
          >
            Cancel
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={saving}
            onClick={() => void persistDraft()}
          >
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      )}
    </div>
  );
}
