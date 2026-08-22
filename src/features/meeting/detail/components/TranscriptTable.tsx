import { memo, type KeyboardEvent, type ReactNode } from "react";

import { cn } from "@/shared/lib/utils";
import type { TranscriptLayout } from "@/shared/lib/types/pipeline";
import { notesSegmentText } from "@/features/pipeline/lib/sessionMode";
import {
  connectionGapRowClass,
  notesFocusedRowClass,
  notesRowClass,
  notesTextClass,
  sideBySideFocusedRowClass,
  sideBySideRowClass,
  sideBySideSourceTextClass,
  sideBySideTranslatedTextClass,
  stackedBlockClass,
  stackedFocusedRowClass,
  stackedRowClass,
  stackedSourceTextClass,
  stackedTranslatedTextClass,
  transcriptCellClass,
  transcriptHeaderCellClass,
  transcriptHeaderClass,
  transcriptLiveAccentClass,
  transcriptRowInteractiveClass,
} from "@/features/pipeline/lib/transcriptLayoutStyles";

export type TranscriptRowVariant = "bilingual" | "notes";

export function TranscriptTableHeader({
  layout,
  variant = "bilingual",
}: {
  layout: TranscriptLayout;
  variant?: TranscriptRowVariant;
}) {
  if (variant === "notes" || layout === "stacked") return null;

  return (
    <div className={transcriptHeaderClass} role="row">
      <span className={transcriptHeaderCellClass} role="columnheader">
        Original
      </span>
      <span className={transcriptHeaderCellClass} role="columnheader">
        Translation
      </span>
    </div>
  );
}

interface TranscriptRowProps {
  layout: TranscriptLayout;
  variant?: TranscriptRowVariant;
  source?: string;
  translated?: string;
  connectionGap?: boolean;
  focused?: boolean;
  live?: boolean;
  interactive?: boolean;
  dataSegmentId?: string;
  /** Optional meta row inside the segment shell (e.g. time · speaker). */
  header?: ReactNode;
  onClick?: () => void;
  onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void;
}

function renderSideBySideBody(focused?: boolean, source?: string, translated?: string) {
  return (
    <>
      <p className={cn(transcriptCellClass, sideBySideSourceTextClass(focused))}>
        {source?.trim() ? source : "\u00a0"}
      </p>
      <p className={cn(transcriptCellClass, sideBySideTranslatedTextClass(focused))}>
        {translated?.trim() ? translated : "\u00a0"}
      </p>
    </>
  );
}

function renderStackedBody(focused?: boolean, source?: string, translated?: string) {
  return (
    <div className={stackedBlockClass}>
      <p className={stackedSourceTextClass(focused)}>
        {source?.trim() ? source : "\u00a0"}
      </p>
      <p className={stackedTranslatedTextClass(focused)}>
        {translated?.trim() ? translated : "\u00a0"}
      </p>
    </div>
  );
}

function renderNotesBody(focused?: boolean, source?: string, translated?: string) {
  const text = notesSegmentText(source, translated);
  return <p className={notesTextClass(focused)}>{text ? text : "\u00a0"}</p>;
}

export const TranscriptRow = memo(function TranscriptRow({
  layout,
  variant = "bilingual",
  source,
  translated,
  connectionGap,
  focused,
  live,
  interactive,
  dataSegmentId,
  header,
  onClick,
  onKeyDown,
}: TranscriptRowProps) {
  // Live rows match history text/background; accent border marks the streaming tail.
  const emphasized = focused;
  const notes = variant === "notes";
  const body: ReactNode = notes
    ? renderNotesBody(emphasized, source, translated)
    : layout === "stacked"
      ? renderStackedBody(emphasized, source, translated)
      : renderSideBySideBody(emphasized, source, translated);

  const content = header ? (
    <div className="flex min-w-0 flex-col gap-1.5">
      {header}
      {body}
    </div>
  ) : (
    body
  );

  const rowClass = cn(
    notes ? notesRowClass : layout === "stacked" ? stackedRowClass : sideBySideRowClass,
    live && transcriptLiveAccentClass,
    connectionGap && connectionGapRowClass(layout, variant),
    focused &&
      !live &&
      (notes
        ? notesFocusedRowClass
        : layout === "stacked"
          ? stackedFocusedRowClass
          : sideBySideFocusedRowClass),
    interactive && !focused && !live && transcriptRowInteractiveClass,
  );

  if (interactive) {
    return (
      <div
        data-segment-id={dataSegmentId}
        className={rowClass}
        onClick={onClick}
        onKeyDown={onKeyDown}
        role="button"
        tabIndex={0}
      >
        {content}
      </div>
    );
  }

  return <div className={rowClass}>{content}</div>;
});
