/**
* Shared helpers for the 4-section meeting-context payload (general pairs /
* background text / recognition terms / translation pairs). Used by the
* Soniox STT context settings (live session) and the app-level Meeting
* context editor for summaries.
*/
import type {
  SonioxContextPayload,
  SonioxGeneralPair,
  SonioxTranslationTerm,
} from "@/shared/lib/types/pipeline";

/** Trim + drop blank rows before persisting a context payload. */
export function cleanPayload(payload: SonioxContextPayload): SonioxContextPayload {
  return {
    general: payload.general
      .filter((p) => p.key.trim() && p.value.trim())
      .map((p) => ({ key: p.key.trim(), value: p.value.trim() })),
    text: payload.text.trim(),
    terms: payload.terms.map((t) => t.trim()).filter(Boolean),
    translationTerms: payload.translationTerms
      .filter((t) => t.source.trim() && t.target.trim())
      .map((t) => ({
        source: t.source.trim(),
        target: t.target.trim(),
      })),
  };
}

/** Flat preset keys — type freely for custom, or pick from the list. */
export const GENERAL_KEY_PRESETS: { value: string; label: string }[] = [
  { value: "domain", label: "Domain" },
  { value: "topic", label: "Topic" },
  { value: "meeting_type", label: "Meeting type" },
  { value: "speakers", label: "Speakers" },
  { value: "roles", label: "Roles" },
  { value: "organization", label: "Organization" },
  { value: "product", label: "Product" },
];

export function presetLabel(key: string): string | undefined {
  return GENERAL_KEY_PRESETS.find((p) => p.value === key)?.label;
}

export type IdPair = SonioxTranslationTerm & { id: string };
export type IdGeneral = SonioxGeneralPair & { id: string };

function withTermIds(pairs: SonioxTranslationTerm[]): IdPair[] {
  return pairs.map((p, i) => ({
    ...p,
    id: `term-${i}-${p.source}-${p.target}`,
  }));
}

function withGeneralIds(pairs: SonioxGeneralPair[]): IdGeneral[] {
  return pairs.map((p, i) => ({
    ...p,
    id: `gen-${i}-${p.key}`,
  }));
}

/** Always show at least one empty key–value row so new/empty editors are self-explanatory. */
export function ensureGeneralRows(
  pairs: SonioxGeneralPair[],
  emptyId: string,
): IdGeneral[] {
  const rows = withGeneralIds(pairs);
  if (rows.length > 0) return rows;
  return [{ id: emptyId, key: "", value: "" }];
}

export function ensureTranslationRows(
  pairs: SonioxTranslationTerm[],
  emptyId: string,
): IdPair[] {
  const rows = withTermIds(pairs);
  if (rows.length > 0) return rows;
  return [{ id: emptyId, source: "", target: "" }];
}
