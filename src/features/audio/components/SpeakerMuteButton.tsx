import { Volume2, VolumeX } from "lucide-react";
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

export default function SpeakerMuteButton({
  muted,
  disabled,
  onToggle,
  title = "Mute meeting audio (app playback)",
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
        aria-label="Mute meeting audio"
        disabled={disabled}
        onClick={onToggle}
      >
        {muted ? (
          <VolumeX className="size-[1.125rem]" aria-hidden />
        ) : (
          <Volume2 className="size-[1.125rem]" aria-hidden />
        )}
      </Button>
    </AppTooltip>
  );
}
