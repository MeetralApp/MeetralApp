/**
* Window CustomEvent — citation chip → Live transcript column focus.
* Cross-tree transcript focus without lifting state through TranslateView.
*/

export const TRANSCRIPT_FOCUS_EVENT = "meetral:transcript-focus";

export type TranscriptFocusDetail = {
  segmentId: string;
  /** When set, only the matching column handles the jump. */
  direction?: "outbound" | "inbound";
};

export function dispatchTranscriptFocus(detail: TranscriptFocusDetail): void {
  const segmentId = detail.segmentId.trim();
  if (!segmentId) return;
  window.dispatchEvent(
    new CustomEvent<TranscriptFocusDetail>(TRANSCRIPT_FOCUS_EVENT, {
      detail: {
        segmentId,
        direction: detail.direction,
      },
    }),
  );
}

export function listenTranscriptFocus(
  handler: (detail: TranscriptFocusDetail) => void,
): () => void {
  const listener = (event: Event) => {
    const detail = (event as CustomEvent<TranscriptFocusDetail>).detail;
    if (!detail?.segmentId?.trim()) return;
    handler(detail);
  };
  window.addEventListener(TRANSCRIPT_FOCUS_EVENT, listener);
  return () => window.removeEventListener(TRANSCRIPT_FOCUS_EVENT, listener);
}
