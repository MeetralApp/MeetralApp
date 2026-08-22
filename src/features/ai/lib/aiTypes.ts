export type AiProvider = "gemini" | "openAi" | "soniox";

export interface LanguageInfo {
  code: string;
  name: string;
  countryCode: string;
}

export interface AiModelInfo {
  id: string;
  label: string;
  description: string;
}

/** Persisted live/STT model + languages it supports (per-provider catalogs). */
export interface LiveModelOption {
  id: string;
  name?: string;
  languages: LanguageInfo[];
}

/** Persisted Soniox TTS model + languages (voices stay in sonioxTtsVoices). */
export interface SonioxTtsModelOption {
  id: string;
  name?: string;
  languages: LanguageInfo[];
}

export interface AiCatalogDefaults {
  liveModel: string;
  summaryModel: string;
}

export interface ProviderCapabilities {
  supportsVadConfig: boolean;
  supportsEchoTargetLanguage: boolean;
  liveUploadSampleRate: number;
  supportsNativeSummary?: boolean;
  usesSeparateTts?: boolean;
  /** Notes mode needs STT-only (no MT). Soniox true; Gemini/OpenAI false. */
  supportsNotesSttOnly?: boolean;
}

export interface ProviderCatalog {
  provider: AiProvider;
  languages: LanguageInfo[];
  liveModels: AiModelInfo[];
  summaryModels: AiModelInfo[];
  defaults: AiCatalogDefaults;
  capabilities: ProviderCapabilities;
  /** OpenAI Notes: fixed Realtime transcription model (not in liveModels). */
  notesSttModel?: AiModelInfo | null;
}

export function providerLabel(provider: AiProvider): string {
  if (provider === "openAi") return "OpenAI";
  if (provider === "soniox") return "Soniox";
  return "Gemini";
}
