import { useState, type ReactNode } from "react";
import {
  AlertTriangle,
  CircleCheck,
  Info,
  RefreshCw,
  X,
} from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

export type LiveNoticeTone = "warning" | "destructive" | "neutral";

type LiveNoticeIconPreset = "warning" | "reconnecting" | "restored" | "info";

export type LiveNoticeIcon = LiveNoticeIconPreset | null | ReactNode;

export type LiveNoticeActionSpec = {
  label: string;
  onClick: () => void | Promise<void>;
  intent?: "primary" | "secondary";
  disabled?: boolean;
  loading?: boolean;
};

const TONE_SURFACE: Record<LiveNoticeTone, string> = {
  warning: "border-warning/25 bg-warning/5",
  destructive: "border-destructive/30 bg-destructive/10",
  neutral: "border-border bg-secondary/30",
};

const ICON_TONE: Record<LiveNoticeTone, string> = {
  warning: "text-warning",
  destructive: "text-destructive",
  neutral: "text-muted-foreground",
};

const ACTION_BASE =
  "h-7 shrink-0 px-2 text-xs font-medium hover:bg-hover-surface";

const ACTION_INTENT = {
  primary: "text-accent hover:text-accent",
  secondary: "text-muted-foreground hover:text-foreground",
} as const;

function resolveIcon(
  icon: LiveNoticeIcon | undefined,
  tone: LiveNoticeTone,
  fallbackPreset: LiveNoticeIconPreset | null,
): ReactNode | null {
  if (icon === null) return null;
  const resolved = icon === undefined ? fallbackPreset : icon;
  if (resolved == null) return null;
  if (typeof resolved !== "string") {
    return (
      <span className="shrink-0 [&_svg]:size-3.5">{resolved}</span>
    );
  }
  const className = cn("size-3.5 shrink-0", ICON_TONE[tone]);
  switch (resolved) {
    case "warning":
      return <AlertTriangle className={className} aria-hidden strokeWidth={2} />;
    case "reconnecting":
      return (
        <RefreshCw
          className={cn(className, "animate-spin")}
          aria-hidden
          strokeWidth={2}
        />
      );
    case "restored":
      return <CircleCheck className={className} aria-hidden strokeWidth={2} />;
    case "info":
      return <Info className={className} aria-hidden strokeWidth={2} />;
  }
}

function StripAction({ action }: { action: LiveNoticeActionSpec }) {
  const [pending, setPending] = useState(false);
  const busy = Boolean(action.loading || pending);
  const intent = action.intent ?? "primary";

  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      disabled={Boolean(action.disabled || busy)}
      className={cn(ACTION_BASE, ACTION_INTENT[intent])}
      onClick={() => {
        if (busy) return;
        const result = action.onClick();
        if (result && typeof (result as Promise<void>).then === "function") {
          setPending(true);
          void Promise.resolve(result).finally(() => setPending(false));
        }
      }}
    >
      {action.label}
    </Button>
  );
}

export type LiveNoticeStripProps = {
  tone?: LiveNoticeTone;
  role?: "status" | "alert";
  icon?: LiveNoticeIcon;
  /** Plain body (Setup, fatal). Ignored when `title` is set. */
  message?: string;
  /** Device-unavailable primary label. */
  title?: string;
  /** Truncated secondary line; full string in tooltip. */
  detail?: string | null;
  actions?: LiveNoticeActionSpec[];
  className?: string;
};

function LiveNoticeStrip({
  tone = "warning",
  role,
  icon,
  message,
  title,
  detail,
  actions,
  className,
}: LiveNoticeStripProps) {
  const resolvedRole = role ?? (tone === "destructive" ? "alert" : "status");
  const resolvedIcon = resolveIcon(
    icon,
    tone,
    tone === "neutral" ? null : "warning",
  );
  const bodyTone =
    tone === "destructive" ? "text-destructive" : "text-muted-foreground";
  const fullDetail = detail?.trim() || null;
  const showTitleRow = Boolean(title?.trim());

  return (
    <div
      role={resolvedRole}
      className={cn(
        "flex items-center gap-2 rounded-md border px-2.5 py-1.5 text-xs leading-snug",
        TONE_SURFACE[tone],
        className,
      )}
    >
      {resolvedIcon}
      <div className="min-w-0 flex-1">
        {showTitleRow ? (
          <div className="flex min-w-0 items-baseline gap-1.5">
            <span className={cn("shrink-0 font-medium", bodyTone)}>
              {title}
            </span>
            {fullDetail ? (
              <>
                <span className="shrink-0 text-muted-foreground/50" aria-hidden>
                  ·
                </span>
                <AppTooltip label={fullDetail}>
                  <span className={cn("min-w-0 truncate", bodyTone)}>
                    {fullDetail}
                  </span>
                </AppTooltip>
              </>
            ) : null}
          </div>
        ) : (
          <p className={bodyTone}>{message}</p>
        )}
      </div>
      {actions && actions.length > 0 ? (
        <div className="flex shrink-0 items-center gap-0.5">
          {actions.map((action) => (
            <StripAction key={action.label} action={action} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export type LiveNoticeRailProps = {
  tone?: LiveNoticeTone;
  icon?: LiveNoticeIcon;
  label: string;
  meta?: string | null;
  detail?: string | null;
  ariaLabel: string;
  onDismiss?: () => void;
  dismissLabel?: string;
  className?: string;
};

function LiveNoticeRail({
  tone = "warning",
  icon = "warning",
  label,
  meta,
  detail,
  ariaLabel,
  onDismiss,
  dismissLabel = "Dismiss",
  className,
}: LiveNoticeRailProps) {
  const resolvedIcon = resolveIcon(icon, tone, "warning");
  const textTone =
    tone === "destructive"
      ? "text-destructive"
      : tone === "neutral"
        ? "text-muted-foreground"
        : "text-warning";

  return (
    <div
      role="status"
      aria-label={ariaLabel}
      className={cn(
        "flex min-h-8 items-center gap-2 border-b px-3 py-1.5 text-xs",
        TONE_SURFACE[tone],
        textTone,
        className,
      )}
    >
      {resolvedIcon}
      <AppTooltip label={detail ?? label}>
        <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
      </AppTooltip>
      {meta ? (
        <span className="shrink-0 tabular-nums opacity-80">{meta}</span>
      ) : null}
      {onDismiss ? (
        <AppTooltip label={dismissLabel}>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="size-6 shrink-0 text-inherit hover:bg-hover-surface"
            onClick={onDismiss}
            aria-label={dismissLabel}
          >
            <X className="size-3.5" aria-hidden strokeWidth={2} />
          </Button>
        </AppTooltip>
      ) : null}
    </div>
  );
}

const LiveNotice = {
  Strip: LiveNoticeStrip,
  Rail: LiveNoticeRail,
};

export default LiveNotice;
