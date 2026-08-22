import AppTooltip from "@/shared/components/AppTooltip";
import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import FlagIcon from "./FlagIcon";
import type { Language } from "../lib/languages";
import { FALLBACK_LANGUAGES } from "../lib/languages";

interface Props {
  id?: string;
  label: string;
  value: string;
  onChange: (code: string) => void;
  disabled?: boolean;
  disabledTitle?: string;
  languages?: Language[];
}

export default function LanguagePicker({
  id,
  label,
  value,
  onChange,
  disabled = false,
  disabledTitle = "Stop all pipelines to change languages",
  languages = FALLBACK_LANGUAGES,
}: Props) {
  const options = languages;

  return (
    <div className="flex min-w-0 flex-col gap-1.5">
      <Label htmlFor={id} className={settingsFieldLabelClass}>
        {label}
      </Label>
      <Select value={value} onValueChange={onChange} disabled={disabled}>
        <AppTooltip label={disabled ? disabledTitle : ""}>
          <span className="block w-full">
            <SelectTrigger id={id} className="w-full" aria-label={label}>
              <SelectValue placeholder="Select language" />
            </SelectTrigger>
          </span>
        </AppTooltip>
        <SelectContent position="popper" className="max-h-60">
          {options.map((lang) => (
            <SelectItem key={lang.code} value={lang.code}>
              <span className="flex items-center gap-2">
                <FlagIcon
                  languageCode={lang.code}
                  countryCode={lang.countryCode}
                  className="size-4 shrink-0"
                />
                <span>{lang.name}</span>
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
