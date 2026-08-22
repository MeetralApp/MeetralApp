import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";
import {
  ANCHOR_TIMER_BADGE_CLASS,
  ANCHOR_TIMER_BADGE_INTERACTIVE_CLASS,
  anchorBadgeFullTitle,
  anchorDirectionDotClass,
} from "@/shared/lib/anchorBadgeStyle";

export type AnchorTimeBadgeProps = {
  /** Formatted timestamp (caller formats, e.g. "1:05"). */
  timer: string;
  /** Direction dot semantics: outbound = You (primary), else Meeting (muted). */
  direction?: string | null;
  /** Speaker label for tooltip/aria ("You" / "Meeting" / custom). */
  speaker?: string;
  /** Segment snippet — enriches the tooltip when the surface can resolve one. */
  snippet?: string;
  /** When absent the badge renders as a static pill with a native title. */
  onClick?: () => void;
  className?: string;
};

/**
* The one anchor-timer badge (design-system/MASTER.md): direction dot + timestamp pill,
* tooltip speaker · time (+ snippet), optional jump click. Used by summary TipTap
* citations and artifact chips — not a chat surface.
*/
export function AnchorTimeBadge({
  timer,
  direction,
  speaker,
  snippet,
  onClick,
  className,
}: AnchorTimeBadgeProps) {
  const classes = cn(
    ANCHOR_TIMER_BADGE_CLASS,
    onClick && ANCHOR_TIMER_BADGE_INTERACTIVE_CLASS,
    className,
  );
  const content = (
    <>
      <span aria-hidden className={anchorDirectionDotClass(direction)} />
      {timer}
    </>
  );

  if (!onClick) {
    return (
      <span
        className={classes}
        title={anchorBadgeFullTitle(timer, speaker, snippet)}
      >
        {content}
      </span>
    );
  }

  const ariaLabel = `Jump to ${timer}${speaker ? ` · ${speaker}` : ""}`;
  const tip = snippet ? (
    <span className="block max-w-60 space-y-1">
      <span className="block font-medium">
        {speaker} · {timer}
      </span>
      <span className="block whitespace-pre-line text-muted-foreground line-clamp-4">
        {snippet}
      </span>
    </span>
  ) : (
    ariaLabel
  );
  return (
    <AppTooltip label={tip} contentClassName="max-w-64">
      <button
        type="button"
        className={classes}
        onClick={onClick}
        aria-label={ariaLabel}
      >
        {content}
      </button>
    </AppTooltip>
  );
}
