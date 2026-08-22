import { Mic, MicOff } from "lucide-react";
import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

interface Props {
  muted: boolean;
  disabled?: boolean;
  onToggle: () => void;
  /** Hover label; `null` disables tooltip. */
  title?: string | null;
}

export default function MicMuteButton({
  muted,
  disabled,
  onToggle,
  title = "Mute in meeting (app relay)",
}: Props) {
  return (
    <AppTooltip label={title}>
      <Button
        type="button"
        variant="outline"
        size="icon"
        className={cn(
          // Match Direct/Translate path-track outer height (h-8 + p-0.5).
          "size-9 shrink-0 rounded-md border border-border bg-card text-foreground shadow-none",
          "hover:bg-hover-surface dark:border-border dark:bg-card dark:hover:bg-hover-surface",
          muted &&
            "border-destructive bg-destructive/20 text-destructive hover:bg-destructive/30 hover:text-destructive dark:border-destructive dark:bg-destructive/20",
        )}
        aria-pressed={muted}
        aria-label="Mute voice in meeting"
        disabled={disabled}
        onClick={onToggle}
      >
        {muted ? (
          <MicOff className="size-[1.125rem]" aria-hidden />
        ) : (
          <Mic className="size-[1.125rem]" aria-hidden />
        )}
      </Button>
    </AppTooltip>
  );
}
