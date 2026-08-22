/** Pipeline toolbar accent colors (Direct / Translate / Notes) — theme via CSS vars. */

/**
* Active path pill: domain wash + strong elevation (default = hover-level shadow)
* so active always outranks idle hover.
*/
const segmentActiveBase =
  "border-0 transition-[background-color,box-shadow,color,opacity] duration-150 ease-out [box-shadow:var(--pipeline-path-pill-shadow-hover)]";

/**
* Segmented track — recessed well (track bg + inset + ring) both themes;
* tokens in `index.css` (`--pipeline-path-track-*`).
*/
export const pipelinePathTrackClass =
  "inline-flex w-max max-w-[17rem] min-w-0 shrink items-center gap-0.5 rounded-lg bg-[var(--pipeline-path-track-bg)] p-0.5 ring-1 ring-border/50 [box-shadow:var(--pipeline-path-track-shadow)]";

export const pipelineToolbarClasses = {
  /**
  * Idle: muted text only. Hover brightens text — no white elevated pill
  * (that was competing with / overpowering the active domain wash).
  */
  segmentIdle:
    "border-0 bg-transparent text-foreground/55 shadow-none transition-[background-color,color,opacity] duration-150 ease-out hover:bg-foreground/[0.06] hover:text-foreground",
  /** Active Direct: green wash + strong elevation. */
  directActive: `${segmentActiveBase} bg-[var(--pipeline-direct-bg)] text-[var(--pipeline-direct-text)] hover:bg-[var(--pipeline-direct-bg)] hover:text-[var(--pipeline-direct-text)]`,
  /** Active Translate: cyan wash + strong elevation. */
  translateActive: `${segmentActiveBase} bg-[var(--pipeline-translate-bg)] text-[var(--pipeline-translate-text)]`,
  /** Active Translate group: keep wash; shadow already at hover strength. */
  translateActiveHover:
    "hover:bg-[var(--pipeline-translate-bg)] hover:text-[var(--pipeline-translate-text)]",
  /** Active Notes: amber wash — distinct from Translate cyan. */
  notesActive: `${segmentActiveBase} bg-[var(--pipeline-notes-bg)] text-[var(--pipeline-notes-text)]`,
  notesActiveHover:
    "hover:bg-[var(--pipeline-notes-bg)] hover:text-[var(--pipeline-notes-text)]",
  /**
  * Gutter between elevated pane cards — hit target only; the grip icon is
  * centered in TranscriptResizeHandle (no full-height hairline).
  */
  resizeHandle: "relative w-3 shrink-0 cursor-col-resize bg-transparent",
  /** Live / detail column shell — elevation via --pane-elevated tokens.
  * Live accent is on the transcript row (`transcriptLiveAccentClass`), not the pane. */
  paneSurface:
    "rounded-[10px] bg-card [box-shadow:var(--pane-elevated)]",
  /** Toolbar strip inside elevated pane — opaque secondary both themes. */
  paneToolbarRail: "border-b border-border bg-secondary",
  /**
  * Icon action chip on `paneToolbarRail` (and similar secondary strips).
  * Light: white `card` on gray rail. Dark: `hover-surface` — lighter than
  * secondary `#1e2430` (do not use `bg-card` in dark; it sinks into the rail).
  */
  paneToolbarActionChip:
    "inline-flex size-8 shrink-0 items-center justify-center gap-0 !p-0 border-border bg-card text-foreground shadow-sm hover:bg-hover-surface dark:border-white/20 dark:!bg-hover-surface dark:shadow-[0_1px_0_rgba(255,255,255,0.1)] dark:hover:!bg-[#2e3545] [&_svg]:size-3.5 [&_svg]:shrink-0",
} as const;
