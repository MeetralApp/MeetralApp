import { invoke } from "@tauri-apps/api/core";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import type {
  CustomLlmProfileInput,
  CustomLlmProfileView,
  SaveConfigPayload,
  SaveConfigResult,
} from "@/shared/lib/types/pipeline";

export async function saveConfig(
  config: SaveConfigPayload,
): Promise<SaveConfigResult> {
  return invoke<SaveConfigResult>("save_config", { config });
}

export async function testApiKey(
  apiKey: string,
  provider: AiProvider,
  purpose: "live" | "summary" = "live",
): Promise<void> {
  await invoke("test_api_key", { request: { apiKey, provider, purpose } });
}

// ---------------------------------------------------------------------------
// Custom OpenAI-compatible LLM profiles
// ---------------------------------------------------------------------------

export async function upsertCustomLlmProfile(
  request: CustomLlmProfileInput,
): Promise<CustomLlmProfileView> {
  return invoke<CustomLlmProfileView>("upsert_custom_llm_profile", { request });
}

export async function deleteCustomLlmProfile(id: string): Promise<void> {
  await invoke("delete_custom_llm_profile", { id });
}

/** Dry-run connection test (JSON-mode generate + streaming chat completions). */
export async function testCustomLlmProfile(
  request: CustomLlmProfileInput,
): Promise<void> {
  await invoke("test_custom_llm_profile", { request });
}
