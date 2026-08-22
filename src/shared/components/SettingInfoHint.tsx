import type { ReactNode } from "react";
import { CircleHelp } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

interface SettingInfoHintProps {
  label: string;
  children: ReactNode;
  side?: "top" | "right" | "bottom" | "left";
  contentClassName?: string;
}

export default function SettingInfoHint({
  label,
  children,
  side = "top",
  contentClassName,
}: SettingInfoHintProps) {
  return (
    <AppTooltip
      label={
        contentClassName ? (
          <span className={cn(contentClassName)}>{children}</span>
        ) : (
          children
        )
      }
      side={side}
    >
      <Button
        type="button"
        variant="ghost"
        size="icon-xs"
        className="size-5 shrink-0 rounded-full text-muted-foreground"
        aria-label={label}
        onClick={(e) => e.stopPropagation()}
      >
        <CircleHelp className="size-3.5" aria-hidden />
      </Button>
    </AppTooltip>
  );
}
