import type { ConfigView } from "@/shared/lib/types/pipeline";
import {
  emptySonioxContextPayload,
  estimateSonioxPayloadChars,
  mergeSonioxContextPreview,
  SONIOX_CONTEXT_CHAR_BUDGET,
  type SonioxContextPayload,
  type SonioxContextProfile,
} from "@/shared/lib/types/pipeline";

// Row/preset helpers moved to shared — re-export so existing
// consumers keep one import site.
export {
  cleanPayload,
  ensureGeneralRows,
  ensureTranslationRows,
  GENERAL_KEY_PRESETS,
  presetLabel,
  type IdGeneral,
  type IdPair,
} from "@/shared/lib/contextPayload";

/** True when merged always-on + active profile exceeds Soniox char budget. */
export function isSonioxContextOverBudget(config: ConfigView): boolean {
  const alwaysOn = config.sonioxAlwaysOn ?? emptySonioxContextPayload();
  const profiles = config.sonioxContextProfiles ?? [];
  const activeId = config.sonioxActiveContextProfileId ?? null;
  const profile =
    activeId != null
      ? (profiles.find((p) => p.id === activeId) ?? null)
      : null;
  const merged = mergeSonioxContextPreview(alwaysOn, profile);
  return estimateSonioxPayloadChars(merged) > SONIOX_CONTEXT_CHAR_BUDGET;
}

export function payloadSummary(payload: SonioxContextPayload): string {
  const terms = payload.terms.filter((t) => t.trim()).length;
  const general = payload.general.filter(
    (p) => p.key.trim() && p.value.trim(),
  ).length;
  const parts: string[] = [];
  if (general) parts.push(`${general} facts`);
  if (terms) parts.push(`${terms} terms`);
  if (payload.text.trim()) parts.push("background");
  if (payload.translationTerms.some((t) => t.source.trim() && t.target.trim())) {
    parts.push("translations");
  }
  return parts.length ? parts.join(" · ") : "Empty";
}

export function newSonioxProfile(name = "New profile"): SonioxContextProfile {
  return {
    id: crypto.randomUUID(),
    name,
    includeAlwaysOn: true,
    payload: emptySonioxContextPayload(),
  };
}
