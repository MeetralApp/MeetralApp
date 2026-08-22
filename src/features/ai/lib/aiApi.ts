import { invoke } from "@tauri-apps/api/core";

import type {
  AiProvider,
  LiveModelOption,
  ProviderCatalog,
  SonioxTtsModelOption,
} from "./aiTypes";
import type { ConfigView } from "@/shared/lib/types/pipeline";

export async function getAiCatalog(provider: AiProvider): Promise<ProviderCatalog> {
  return invoke<ProviderCatalog>("get_ai_catalog", { provider });
}

export async function seedLiveModelCatalog(
  provider: AiProvider,
): Promise<LiveModelOption[]> {
  return invoke<LiveModelOption[]>("seed_live_model_catalog", { provider });
}

export async function listSonioxSttModels(
  apiKey = "",
): Promise<LiveModelOption[]> {
  return invoke<LiveModelOption[]>("list_soniox_stt_models", {
    request: { apiKey },
  });
}

export async function listSonioxTtsModels(
  apiKey = "",
): Promise<SonioxTtsModelOption[]> {
  return invoke<SonioxTtsModelOption[]>("list_soniox_tts_models", {
    request: { apiKey },
  });
}

export async function migrateConfigForProvider(
  provider: AiProvider,
): Promise<ConfigView> {
  return invoke<ConfigView>("migrate_config_for_provider", { provider });
}

/** Cached live models for the active provider. */
export function liveModelsFromConfig(
  config: ConfigView,
  provider: AiProvider = config.aiProvider,
): LiveModelOption[] {
  if (provider === "openAi") return config.openAiLiveModels ?? [];
  if (provider === "soniox") return config.sonioxLiveModels ?? [];
  return config.geminiLiveModels ?? [];
}

export function languagesForLiveModel(
  models: LiveModelOption[],
  liveModelId: string,
): LiveModelOption["languages"] {
  const match = models.find((m) => m.id === liveModelId);
  return match?.languages ?? [];
}

export function persistKeyForProvider(
  provider: AiProvider,
): "geminiLiveModels" | "openAiLiveModels" | "sonioxLiveModels" {
  if (provider === "openAi") return "openAiLiveModels";
  if (provider === "soniox") return "sonioxLiveModels";
  return "geminiLiveModels";
}
