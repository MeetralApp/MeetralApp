import type { SessionMode } from "@/shared/lib/types/pipeline";

/** True when config or meeting is in Session Mode Notes. */
export function isNotesSession(
  input: { sessionMode?: SessionMode | null } | null | undefined,
): boolean {
  return input?.sessionMode === "notes";
}

/** Prefer source; fall back to translated (Notes may copy source into translated). */
export function notesSegmentText(
  source?: string | null,
  translated?: string | null,
): string {
  const s = source?.trim() ?? "";
  if (s) return s;
  return translated?.trim() ?? "";
}
