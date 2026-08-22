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
import { OUTPUT_OPTIONS } from "@/features/voice/lib/voiceSettings";
import type { InboundVoiceOutput } from "@/shared/lib/types/pipeline";

interface Props {
  outputMode: InboundVoiceOutput;
  inboundLocked: boolean;
  modeSaving: boolean;
  onPersistMode: (mode: InboundVoiceOutput) => void;
}

export default function InboundVoiceModeSelect({
  outputMode,
  inboundLocked,
  modeSaving,
  onPersistMode,
}: Props) {
  return (
    <SettingsField
      labelContent={
        <span className="inline-flex items-center gap-1.5">
          <Label
            htmlFor="inbound-voice-output-mode"
            className={settingsFieldLabelClass}
          >
            What you hear
          </Label>
          <SettingInfoHint label="About Meeting → You voice">
            Select the speech engine used for translated Meeting → You audio.
          </SettingInfoHint>
        </span>
      }
    >
      <Select
        value={outputMode}
        onValueChange={(value) =>
          onPersistMode(value as InboundVoiceOutput)
        }
        disabled={inboundLocked || modeSaving}
      >
        <SelectTrigger id="inbound-voice-output-mode" className="w-full">
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
      {inboundLocked ? (
        <p className="text-muted-foreground text-xs">
          Stop inbound translate to change Meeting → You voice settings.
        </p>
      ) : null}
    </SettingsField>
  );
}
