import type { ReactNode } from "react";
import {
  AlertTriangle,
  CircleCheck,
  Loader2,
  RefreshCw,
  X,
} from "lucide-react";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import { isDirectionTranslating } from "../lib/pipelineStatus";
import { useBridgeConnectionNotice } from "@/features/audio/hooks/useBridgeConnectionNotice";
import { usePipelineSessionElapsed } from "../hooks/usePipelineSessionElapsed";
import type { ColumnIdleBadgeSnapshot } from "../lib/appState";
import type { ColumnUiState } from "../lib/columnUi";
import type { PipelineDirection } from "../lib/sessionDrift";
import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

interface Props {
  title: string;
  direction: PipelineDirection;
  status: AppStatus | null;
  columnUi: ColumnUiState;
  voiceCloneDegraded?: string | null;
  /** When SetupBanner is visible, hide redundant setup / api-key chips. */
  suppressSetupIdleChip?: boolean;
  onLongReconnectRestored?: (columnTitle: string) => void;
}

/** Compact icon tones — lighter than mute controls so status stays secondary. */
const IDLE_TONE: Record<string, string> = {
  ready: "border-transparent text-success",
  setup: "border-warning/35 text-warning",
  "api-key": "border-warning/35 text-warning",
  "audio-lost": "border-destructive/40 text-destructive",
  "audio-reconnecting": "animate-pulse border-warning/35 text-warning",
  error: "border-destructive/40 text-destructive",
  checking: "border-border/60 text-muted-foreground",
};

function idleIcon(kind: string): ReactNode {
  switch (kind) {
    case "ready":
      return <CircleCheck className="size-3" aria-hidden strokeWidth={2} />;
    case "checking":
      return (
        <Loader2 className="size-3 animate-spin" aria-hidden strokeWidth={2} />
      );
    case "audio-reconnecting":
      return (
        <RefreshCw className="size-3 animate-spin" aria-hidden strokeWidth={2} />
      );
    default:
      return <AlertTriangle className="size-3" aria-hidden strokeWidth={2} />;
  }
}

function StatusIconChip({
  tip,
  ariaLabel,
  tone,
  children,
  className,
}: {
  tip: ReactNode;
  ariaLabel: string;
  tone: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <AppTooltip label={tip}>
      <button
        type="button"
        className="cursor-default border-0 bg-transparent p-0"
        aria-label={ariaLabel}
      >
        <span
          role="status"
          className={cn(
            "inline-flex size-6 shrink-0 items-center justify-center rounded-md border bg-transparent",
            tone,
            className,
          )}
        >
          {children}
        </span>
      </button>
    </AppTooltip>
  );
}

function idleBadgeForColumn(columnUi: ColumnUiState): ColumnIdleBadgeSnapshot {
  return columnUi.idleBadge;
}

export default function ColumnHeaderStatus({
  title,
  direction,
  status,
  columnUi,
  voiceCloneDegraded = null,
  suppressSetupIdleChip = false,
  onLongReconnectRestored,
}: Props) {
  const translating = isDirectionTranslating(status, direction);
  const { mode, dismiss } = useBridgeConnectionNotice(status, direction, {
    onLongReconnectRestored,
  });
  const timer = usePipelineSessionElapsed(status, direction);

  if (mode === "reconnecting" && timer.variant !== "reconnecting") {
    return (
      <StatusIconChip
        tip="Reconnecting…"
        ariaLabel={`${title} translation reconnecting`}
        tone="animate-pulse border-warning/35 text-warning"
      >
        <RefreshCw className="size-3 animate-spin" aria-hidden strokeWidth={2} />
      </StatusIconChip>
    );
  }

  if (mode === "reconnected") {
    return (
      <div className="inline-flex items-center gap-0.5">
        <StatusIconChip
          tip="Connection restored"
          ariaLabel={`${title} connection restored`}
          tone="border-warning/35 text-warning"
        >
          <CircleCheck className="size-3" aria-hidden strokeWidth={2} />
        </StatusIconChip>
        <AppTooltip label="Dismiss">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="size-6 rounded-md text-warning hover:bg-hover-surface"
            onClick={dismiss}
            aria-label="Dismiss connection notice"
          >
            <X className="size-3" aria-hidden strokeWidth={2} />
          </Button>
        </AppTooltip>
      </div>
    );
  }

  if (translating) {
    if (voiceCloneDegraded) {
      return (
        <StatusIconChip
          tip={voiceCloneDegraded}
          ariaLabel={`${title} custom voice unavailable`}
          tone="border-destructive/40 text-destructive"
        >
          <AlertTriangle className="size-3" aria-hidden strokeWidth={2} />
        </StatusIconChip>
      );
    }

    // Starting/stopping + active elapsed already paint inside the
    // Direct/Translate button group — do not duplicate spinner / custom-voice chips.
    if (
      timer.variant === "starting" ||
      timer.variant === "stopping" ||
      timer.variant === "elapsed"
    ) {
      return null;
    }

    if (timer.variant === "reconnecting") {
      return (
        <StatusIconChip
          tip={timer.title ?? "Reconnecting…"}
          ariaLabel={`${title} reconnecting`}
          tone="animate-pulse border-warning/35 text-warning"
        >
          <RefreshCw
            className="size-3 animate-spin"
            aria-hidden
            strokeWidth={2}
          />
        </StatusIconChip>
      );
    }

    return null;
  }

  const idleBadge = idleBadgeForColumn(columnUi);
  // Ready is implied when path controls are enabled — hide chip to reduce chrome.
  if (idleBadge.kind === "ready") {
    return null;
  }
  // Global SetupBanner already explains setup / API key — don't double-signal.
  if (
    suppressSetupIdleChip &&
    (idleBadge.kind === "setup" || idleBadge.kind === "api-key")
  ) {
    return null;
  }

  const tone = IDLE_TONE[idleBadge.kind] ?? IDLE_TONE.checking;

  return (
    <StatusIconChip
      tip={idleBadge.title}
      ariaLabel={`${title} ${idleBadge.label}`}
      tone={tone}
    >
      {idleIcon(idleBadge.kind)}
    </StatusIconChip>
  );
}
