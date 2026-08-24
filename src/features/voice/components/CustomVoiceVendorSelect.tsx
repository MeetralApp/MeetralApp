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
import { CUSTOM_VOICE_VENDOR_OPTIONS } from "@/features/voice/lib/voiceSettings";
import {
  normalizeCustomVoiceVendor,
  type CustomVoiceVendor,
} from "@/shared/lib/types/pipeline";

interface Props {
  id: string;
  vendor: CustomVoiceVendor;
  locked: boolean;
  saving: boolean;
  onPersist: (vendor: CustomVoiceVendor) => void;
}

export default function CustomVoiceVendorSelect({
  id,
  vendor,
  locked,
  saving,
  onPersist,
}: Props) {
  return (
    <SettingsField
      labelContent={
        <span className="inline-flex items-center gap-1.5">
          <Label htmlFor={id} className={settingsFieldLabelClass}>
            Custom voice engine
          </Label>
          <SettingInfoHint label="About custom voice engine">
            ElevenLabs, Fish Audio, or xAI for this column. Train or clone the
            voice on the vendor site, then pick the voice id here. You and
            Meeting may use different engines.
          </SettingInfoHint>
        </span>
      }
    >
      <Select
        value={normalizeCustomVoiceVendor(vendor)}
        onValueChange={(value) => onPersist(normalizeCustomVoiceVendor(value))}
        disabled={locked || saving}
      >
        <SelectTrigger id={id} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {CUSTOM_VOICE_VENDOR_OPTIONS.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {locked ? (
        <p className="text-muted-foreground text-xs">
          Stop this column to switch custom voice engine.
        </p>
      ) : null}
    </SettingsField>
  );
}
