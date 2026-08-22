import type { ConfigView, CustomLlmProfileView } from "@/shared/lib/types/pipeline";

/** Built-in summary LLM providers (never Soniox). */
export type SummaryProvider = "gemini" | "openAi";

/**
* Generic LLM selection: built-in provider or custom
* OpenAI-compatible profile. Mirrors the Rust `LlmSelection` resolver.
*/
export type LlmSelectionView =
  | { kind: "builtin"; provider: SummaryProvider }
  | { kind: "custom"; profile: CustomLlmProfileView };

export function resolveSummaryProvider(config: ConfigView): SummaryProvider {
  return config.summaryProvider === "openAi" ? "openAi" : "gemini";
}

/**
* Resolve the active selection. A selected profile that no longer exists
* falls back to the built-in provider (same as the Rust resolver).
*/
export function resolveLlmSelection(config: ConfigView): LlmSelectionView {
  const id = config.summaryCustomProfileId;
  if (id) {
    const profile = (config.customLlmProfiles ?? []).find((p) => p.id === id);
    if (profile) return { kind: "custom", profile };
  }
  return { kind: "builtin", provider: resolveSummaryProvider(config) };
}

export function summaryKeyConfigured(
  config: ConfigView,
  provider: SummaryProvider = resolveSummaryProvider(config),
): boolean {
  if (provider === "openAi") {
    return Boolean(config.openaiApiKeyConfigured);
  }
  return Boolean(config.geminiApiKeyConfigured);
}

/**
* Whether the active selection is usable: built-ins need their API key;
* custom profiles are usable once saved (the save-time test call passed —
* auth is optional for local servers).
*/
export function selectionUsable(
  config: ConfigView,
  selection: LlmSelectionView = resolveLlmSelection(config),
): boolean {
  if (selection.kind === "custom") return true;
  return summaryKeyConfigured(config, selection.provider);
}

/** Display label for the active selection (status chips, logs). */
export function selectionLabel(selection: LlmSelectionView): string {
  return selection.kind === "custom"
    ? selection.profile.label
    : selection.provider === "openAi"
      ? "OpenAI"
      : "Gemini";
}
