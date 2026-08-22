import { Key } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";
import type { SettingsSectionStatus } from "@/features/config/lib/settingsDrawerStatus";

interface Props {
  open: boolean;
  status: SettingsSectionStatus;
  panelId: string;
  onToggle: () => void;
}

const TONE_CLASS: Record<SettingsSectionStatus["tone"], string> = {
  ok: "border-success/40 bg-success/15 text-success hover:bg-success/20",
  warn: "border-warning/40 bg-warning/15 text-warning hover:bg-warning/20",
  muted:
    "border-border bg-secondary text-muted-foreground hover:bg-hover-surface",
};

/**
* Compact header control for Settings sections/groups that need an API key:
* icon-only Key chip with status tone; tooltip + aria-label carry the label.
*/
export default function ApiKeyChip({
  open,
  status,
  panelId,
  onToggle,
}: Props) {
  const actionLabel = open
    ? `Hide API key (${status.label})`
    : `Manage API key (${status.label})`;

  return (
    <AppTooltip label={`API key · ${status.label}`}>
      <button
        type="button"
        className={cn(
          "inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md border outline-none",
          "focus-visible:ring-2 focus-visible:ring-ring",
          TONE_CLASS[status.tone],
        )}
        aria-expanded={open}
        aria-controls={panelId}
        aria-label={actionLabel}
        onClick={onToggle}
      >
        <Key className="size-3.5" aria-hidden strokeWidth={2} />
      </button>
    </AppTooltip>
  );
}
