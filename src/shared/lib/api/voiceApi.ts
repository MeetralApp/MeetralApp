import { invoke } from "@tauri-apps/api/core";
import type {
  ElevenLabsModelOption,
  ElevenLabsVoiceOption,
  SonioxVoiceOption,
} from "@/shared/lib/types/pipeline";

export async function testElevenLabsApiKey(apiKey: string): Promise<void> {
  await invoke("test_elevenlabs_api_key", { request: { apiKey } });
}

export async function listElevenLabsVoices(
  apiKey = "",
): Promise<ElevenLabsVoiceOption[]> {
  return invoke<ElevenLabsVoiceOption[]>("list_elevenlabs_voices", {
    request: { apiKey },
  });
}

export async function listElevenLabsModels(
  apiKey = "",
): Promise<ElevenLabsModelOption[]> {
  return invoke<ElevenLabsModelOption[]>("list_elevenlabs_models", {
    request: { apiKey },
  });
}

export async function validateElevenLabsVoice(
  voiceId: string,
  apiKey = "",
): Promise<void> {
  await invoke("validate_elevenlabs_voice", {
    request: { apiKey, voiceId },
  });
}

export async function previewElevenLabsVoice(
  voiceId: string,
  apiKey = "",
): Promise<void> {
  await invoke("preview_elevenlabs_voice", {
    request: { apiKey, voiceId },
  });
}

export async function listSonioxVoices(
  apiKey = "",
  preferredModel?: string,
): Promise<SonioxVoiceOption[]> {
  return invoke<SonioxVoiceOption[]>("list_soniox_voices", {
    request: { apiKey, preferredModel: preferredModel ?? null },
  });
}

export async function previewSonioxVoice(
  voice: string,
  apiKey = "",
): Promise<void> {
  await invoke("preview_soniox_voice", {
    request: { apiKey, voice },
  });
}
