import { ArrowLeftRight } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import LanguagePicker from "@/features/pipeline/components/LanguagePicker";
import type { Language } from "@/features/pipeline/lib/languages";
import type { ConfigView } from "@/shared/lib/types/pipeline";

interface Props {
  config: ConfigView;
  languages: Language[];
  languagesLocked: boolean;
  onSave: (myLanguage: string, meetingLanguage: string) => Promise<void>;
}

const LOCKED_HINT_INTERPRETER = "Stop translation to change languages";
const LOCKED_HINT_NOTES = "Stop notes to change language";

export default function LanguageSettings({
  config,
  languages,
  languagesLocked,
  onSave,
}: Props) {
  const isOpenAi = config.aiProvider === "openAi";
  const notesMode = config.sessionMode === "notes";
  const lockedHint = notesMode ? LOCKED_HINT_NOTES : LOCKED_HINT_INTERPRETER;
  const pickerLanguages =
    languages.length > 0
      ? languages
      : [
          {
            code: config.myLanguage || "vi",
            name: config.myLanguage || "Vietnamese",
            countryCode: "",
          },
        ];

  return (
    <div className="flex flex-col gap-3">
      {languagesLocked ? (
        <p className="m-0 text-sm text-muted-foreground">{lockedHint}</p>
      ) : null}
      {notesMode ? (
        <p className="m-0 text-xs text-muted-foreground">
          Notes mode uses one language for the whole meeting (no translation).
        </p>
      ) : null}
      {isOpenAi && !notesMode ? (
        <p className="m-0 text-xs text-muted-foreground">
          OpenAI supports a smaller language set than Gemini or Soniox.
        </p>
      ) : null}
      {notesMode ? (
        <LanguagePicker
          id="settings-meeting-language"
          label="Meeting language"
          value={config.myLanguage}
          languages={pickerLanguages}
          disabled={languagesLocked}
          onChange={(code) => onSave(code, code)}
        />
      ) : (
        <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-end gap-2">
          <LanguagePicker
            id="settings-my-language"
            label="You speak"
            value={config.myLanguage}
            languages={pickerLanguages}
            disabled={languagesLocked}
            onChange={(code) => onSave(code, config.meetingLanguage)}
          />
          <AppTooltip label="Swap languages">
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="mb-0.5 size-9 shrink-0"
              disabled={languagesLocked}
              aria-label="Swap languages"
              onClick={() => onSave(config.meetingLanguage, config.myLanguage)}
            >
              <ArrowLeftRight size={16} aria-hidden />
            </Button>
          </AppTooltip>
          <LanguagePicker
            id="settings-meeting-language"
            label="Meeting speaks"
            value={config.meetingLanguage}
            languages={pickerLanguages}
            disabled={languagesLocked}
            onChange={(code) => onSave(config.myLanguage, code)}
          />
        </div>
      )}
    </div>
  );
}
