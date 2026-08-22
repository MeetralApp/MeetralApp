/**
* Shared "anchor timer" badge recipe (design-system/MASTER.md) — the single source for
* every timestamp pill anchoring to a transcript segment. Class strings +
* label text builders only (no React) so both the React `AnchorTimeBadge`
* and the summary-doc citation atom (Tiptap plain DOM) consume it.
*/
export const ANCHOR_TIMER_BADGE_CLASS = [
  "inline-flex items-center gap-1 rounded-md border border-border/80",
  "bg-background/60 px-1.5 py-0.5",
  "text-[0.7rem] font-medium tabular-nums text-muted-foreground",
].join(" ");

export const ANCHOR_TIMER_BADGE_INTERACTIVE_CLASS =
  "transition-colors hover:bg-secondary hover:text-foreground";

/** Direction dot: You (outbound) primary, Meeting (inbound) muted. */
export function anchorDirectionDotClass(
  direction: string | null | undefined,
): string {
  return [
    "size-1.5 rounded-full",
    direction === "outbound" ? "bg-primary" : "bg-muted-foreground/70",
  ].join(" ");
}

/** Native tooltip/title text (no snippet). */
export function anchorBadgeTitle(timer: string, speaker: string): string {
  return `${timer} · ${speaker}`;
}

/** Full native title — the snippet folds in when present (chat chip parity). */
export function anchorBadgeFullTitle(
  timer: string,
  speaker?: string,
  snippet?: string,
): string {
  if (snippet) return `${speaker} · ${timer}\n${snippet}`;
  if (speaker) return anchorBadgeTitle(timer, speaker);
  return timer;
}

/** Inline pill text for contexts without a tooltip (summary-doc atom). */
export function anchorBadgeInlineText(timer: string, speaker: string): string {
  return `${timer}·${speaker}`;
}
