import { Button } from "@/shared/ui/button";
import { ButtonGroup } from "@/shared/ui/button-group";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import type {
  ConfigView,
  SaveConfigPayload,
  SaveConfigResult,
  TranscriptLayout,
} from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import { isNotesSession } from "@/features/pipeline/lib/sessionMode";
import { cn } from "@/shared/lib/utils";

interface Props {
  config: ConfigView;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
}

const LAYOUT_OPTIONS: { value: TranscriptLayout; label: string }[] = [
  { value: "sideBySide", label: "Side by side" },
  { value: "stacked", label: "Stacked" },
];

export default function TranscriptDisplaySettings({ config, onSave }: Props) {
  const notesMode = isNotesSession(config);

  const saveLayout = (transcriptLayout: TranscriptLayout) => {
    if (transcriptLayout === config.transcriptLayout) return;
    void onSave(toSavePayload({ ...config, transcriptLayout }));
  };

  if (notesMode) {
    return (
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-1.5">
          <span className={cn(settingsFieldLabelClass)}>Segment layout</span>
          <SettingInfoHint label="Transcript segment layout">
            Notes mode shows one line of text per segment (no Original /
            Translation columns). Switch to Interpreter to choose side-by-side
            or stacked bilingual layout.
          </SettingInfoHint>
        </div>
        <p className="m-0 text-sm text-muted-foreground">
          Notes shows one line per segment. Bilingual layout applies in
          Interpreter mode.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-1.5">
        <span className={cn(settingsFieldLabelClass)}>Segment layout</span>
        <SettingInfoHint label="Transcript segment layout">
          Side by side shows Original and Translation in two columns. Stacked
          puts Original on top in smaller muted text; Translation appears below
          for easier reading of long meeting text.
        </SettingInfoHint>
      </div>
      <ButtonGroup aria-label="Transcript segment layout">
        {LAYOUT_OPTIONS.map(({ value, label }) => (
          <Button
            key={value}
            type="button"
            size="sm"
            variant={config.transcriptLayout === value ? "default" : "secondary"}
            aria-pressed={config.transcriptLayout === value}
            onClick={() => saveLayout(value)}
          >
            {label}
          </Button>
        ))}
      </ButtonGroup>
    </div>
  );
}
