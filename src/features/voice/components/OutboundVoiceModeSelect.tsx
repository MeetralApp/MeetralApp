import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SettingsField from "@/shared/components/SettingsField";
import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import {
  engineVoiceHint,
  OUTPUT_OPTIONS,
} from "@/features/voice/lib/voiceSettings";
import type { ConfigView, OutboundVoiceOutput } from "@/shared/lib/types/pipeline";

interface Props {
  config: ConfigView;
  outputMode: OutboundVoiceOutput;
  cloneEnabled: boolean;
  outboundLocked: boolean;
  modeSaving: boolean;
  onPersistMode: (mode: OutboundVoiceOutput) => void;
}

export default function OutboundVoiceModeSelect({
  config,
  outputMode,
  cloneEnabled,
  outboundLocked,
  modeSaving,
  onPersistMode,
}: Props) {
  return (
    <SettingsField
      labelContent={
        <span className="inline-flex items-center gap-1.5">
          <Label htmlFor="voice-output-mode" className={settingsFieldLabelClass}>
            How you sound
          </Label>
          <SettingInfoHint label="About You → Meeting voice">
            {engineVoiceHint(config.aiProvider)}
            {cloneEnabled ? " ElevenLabs uses the voice selected below." : ""}
          </SettingInfoHint>
        </span>
      }
    >
      <Select
        value={outputMode}
        onValueChange={(value) =>
          onPersistMode(value as OutboundVoiceOutput)
        }
        disabled={outboundLocked || modeSaving}
      >
        <SelectTrigger id="voice-output-mode" className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {OUTPUT_OPTIONS.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {cloneEnabled ? (
        <p className="text-muted-foreground text-xs leading-relaxed">
          Train a Professional Voice Clone (mixed EN+VI for vi→en).
          {config.aiProvider === "gemini" ? (
            <>
              {" "}
              For short replies, set{" "}
              <strong>Pause before ending speech</strong> to
              400–500&nbsp;ms under{" "}
              <strong>Translate → Advanced — Speech detection</strong>{" "}
              (Gemini only).
            </>
          ) : null}
        </p>
      ) : null}
      {outboundLocked ? (
        <p className="text-muted-foreground text-xs">
          Stop outbound translate to change You → Meeting voice settings.
        </p>
      ) : null}
    </SettingsField>
  );
}
