import type { LucideIcon } from "lucide-react";
import {
  CircleAlert,
  CircleCheck,
  Loader2,
  Mic,
  Save,
} from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";

type Tone = "ok" | "warn" | "muted";

interface Props {
  /** Detail text — shown in tooltip and used as accessible name. */
  label: string;
  tone?: Tone;
  className?: string;
}

const toneClass: Record<Tone, string> = {
  ok: "border-transparent bg-success/20 text-success",
  warn: "border-transparent bg-warning/20 text-warning",
  muted: "border-transparent bg-muted text-muted-foreground",
};

function statusIcon(label: string, tone: Tone): LucideIcon {
  if (label === "Ready" || label === "Clone ready") return CircleCheck;
  if (label === "Engine voice") return Mic;
  if (label === "Not saved") return Save;
  if (label === "Checking…") return Loader2;
  if (label === "Setup needed" || /to set up$/.test(label)) return CircleAlert;
  if (tone === "ok") return CircleCheck;
  if (tone === "warn") return CircleAlert;
  return Loader2;
}

export default function AppBadge({ label, tone = "muted", className }: Props) {
  const Icon = statusIcon(label, tone);
  const spinning = label === "Checking…";

  return (
    <AppTooltip label={label} side="left">
      <button
        type="button"
        aria-label={label}
        className={cn(
          "inline-flex size-6 shrink-0 items-center justify-center rounded-full border text-xs outline-none",
          "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
          toneClass[tone],
          className,
        )}
      >
        <Icon
          className={cn("size-3.5", spinning && "animate-spin")}
          aria-hidden
        />
      </button>
    </AppTooltip>
  );
}
