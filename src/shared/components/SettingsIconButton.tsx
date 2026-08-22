import type { LucideIcon } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

interface Props {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
  /** Spin the icon (e.g. Refresh while loading). */
  busy?: boolean;
  className?: string;
}

/**
* Compact Settings secondary action: icon-only ghost button + tooltip.
* Use for Refresh / Preview on field rows and section headers.
*/
export default function SettingsIconButton({
  label,
  icon: Icon,
  onClick,
  disabled = false,
  busy = false,
  className,
}: Props) {
  return (
    <AppTooltip label={label}>
      <span className="inline-flex">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className={cn("size-7 shrink-0 cursor-pointer", className)}
          disabled={disabled}
          aria-label={label}
          onClick={onClick}
        >
          <Icon
            className={cn("size-3.5", busy && "animate-spin")}
            aria-hidden
          />
        </Button>
      </span>
    </AppTooltip>
  );
}
