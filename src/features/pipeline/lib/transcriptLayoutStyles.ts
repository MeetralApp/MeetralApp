import { cn } from "@/shared/lib/utils";
import type { TranscriptLayout } from "@/shared/lib/types/pipeline";

/**
* Reading-focused layout styles for Original | Translation segments.
* Side-by-side: two columns with divide-x (Original left, Translation right).
* Stacked: original on top (secondary), translation below (primary) — no per-segment labels.
*
* Segment chrome matches LibraryMeetingRow: rounded-lg shells, hover-surface on interaction.
*/

export const transcriptColumnsClass = "grid grid-cols-2 divide-x divide-border/70";

/** Shared rounded shell for every segment row (history, hover, live, focus). */
export const transcriptSegmentShellClass = "rounded-lg";

export const transcriptHeaderClass = cn(
  "sticky top-0 z-[1] border-b border-secondary bg-background",
  "shadow-[0_4px_8px_rgba(0,0,0,0.2)]",
  transcriptColumnsClass,
);

export const transcriptHeaderCellClass =
  "px-4 py-2.5 text-[0.68rem] font-semibold uppercase tracking-wider text-muted-foreground";

export const sideBySideRowClass = cn(
  transcriptSegmentShellClass,
  "mb-0.5",
  transcriptColumnsClass,
);

export const stackedRowClass = cn(
  transcriptSegmentShellClass,
  "mb-0.5 px-4 py-3",
);

/** Notes: single full-width text line (direction comes from column title). */
export const notesRowClass = cn(
  transcriptSegmentShellClass,
  "mb-0.5 px-4 py-3",
);

export const transcriptCellClass = "min-w-0 px-4 py-3";

export const stackedBlockClass = "flex min-w-0 flex-col gap-2";

/** Meeting detail: clickable history segments. */
export const transcriptRowInteractiveClass = cn(
  "cursor-pointer transition-colors",
  "hover:bg-hover-surface",
  "focus-visible:bg-hover-surface focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
);

/**
* Live tail: accent border only — same shell/background as history rows.
* Horizontal inset comes from the list pad (`transcriptListPadClass`), not the border itself.
*/
export const transcriptLiveAccentClass = "border-l-[3px] border-l-accent";

/** Shared list inset so live accent + history shells clear the pane edge equally. */
export const transcriptListPadClass = "px-2 py-2 pb-3";

const transcriptActiveShellClass = cn(
  transcriptSegmentShellClass,
  "mt-1 mb-0.5",
  transcriptLiveAccentClass,
  "bg-secondary/60",
);

export function sideBySideSourceTextClass(focused?: boolean): string {
  return cn(
    "transcript-source m-0 break-words hyphens-auto whitespace-pre-wrap",
    "text-[0.9375rem] leading-[1.7] tracking-[0.01em]",
    focused ? "text-foreground/90" : "text-muted-foreground",
  );
}

export function sideBySideTranslatedTextClass(focused?: boolean): string {
  return cn(
    "transcript-translated m-0 break-words hyphens-auto whitespace-pre-wrap",
    "text-[0.9375rem] leading-[1.7] tracking-[0.01em]",
    focused ? "font-medium text-foreground" : "font-normal text-foreground",
  );
}

export function stackedTranslatedTextClass(focused?: boolean): string {
  return cn(
    "transcript-translated m-0 min-w-0 break-words hyphens-auto whitespace-pre-wrap",
    "text-[0.9375rem] leading-[1.7] tracking-[0.01em]",
    focused ? "font-medium text-foreground" : "font-normal text-foreground",
  );
}

export function stackedSourceTextClass(focused?: boolean): string {
  return cn(
    "transcript-source m-0 min-w-0 break-words hyphens-auto whitespace-pre-wrap",
    "text-[0.9375rem] leading-[1.7] tracking-[0.01em]",
    focused ? "text-muted-foreground" : "text-muted-foreground/90",
  );
}

export const sideBySideFocusedRowClass = cn(
  transcriptActiveShellClass,
  transcriptColumnsClass,
);

export const stackedFocusedRowClass = cn(transcriptActiveShellClass, "px-4 py-3");

export const notesFocusedRowClass = cn(transcriptActiveShellClass, "px-4 py-3");

export function notesTextClass(focused?: boolean): string {
  return cn(
    "transcript-notes m-0 min-w-0 w-full break-words hyphens-auto whitespace-pre-wrap",
    "text-[0.9375rem] leading-[1.7] tracking-[0.01em]",
    focused ? "font-medium text-foreground" : "font-normal text-foreground",
  );
}

export function connectionGapRowClass(
  layout: TranscriptLayout,
  variant: "bilingual" | "notes" = "bilingual",
): string {
  if (variant === "notes") {
    return "[&_.transcript-notes]:text-sm [&_.transcript-notes]:italic [&_.transcript-notes]:text-warning";
  }
  return layout === "stacked"
    ? "[&_.transcript-source]:text-sm [&_.transcript-source]:italic [&_.transcript-source]:text-warning [&_.transcript-translated]:text-sm [&_.transcript-translated]:italic [&_.transcript-translated]:text-warning"
    : "[&_.transcript-source]:text-sm [&_.transcript-source]:italic [&_.transcript-source]:text-warning [&_.transcript-translated]:text-sm [&_.transcript-translated]:italic [&_.transcript-translated]:text-warning";
}
