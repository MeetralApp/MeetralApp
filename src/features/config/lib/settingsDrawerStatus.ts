import type { AiProvider } from "@/features/ai/lib/aiTypes";
import { providerLabel } from "@/features/ai/lib/aiTypes";
import { formatAudioSectionStatus } from "@/features/audio/lib/audioSetup";
import type { AudioSetupValidation } from "@/features/audio/lib/audioSetup";
import { selectionUsable } from "@/features/config/lib/summaryProvider";
import type { ConfigView } from "@/shared/lib/types/pipeline";
import { isCustomVoiceOutput, normalizeCustomVoiceVendor } from "@/shared/lib/types/pipeline";

export type SettingsSectionTone = "ok" | "warn" | "muted";

export interface SettingsSectionStatus {
  tone: SettingsSectionTone;
  label: string;
}

export function translationSectionStatus(
  config: ConfigView,
  apiKeyDirty: boolean,
): SettingsSectionStatus {
  if (!config.apiKeyConfigured) {
    return { tone: "warn", label: "Setup needed" };
  }
  if (apiKeyDirty) {
    return { tone: "warn", label: "Not saved" };
  }
  return { tone: "ok", label: "Ready" };
}

export function intelligenceSectionStatus(config: ConfigView): SettingsSectionStatus {
  return selectionUsable(config)
    ? { tone: "ok", label: "Ready" }
    : { tone: "warn", label: "Setup needed" };
}

export function audioSectionStatus(
  audioDirty: boolean,
  audioSetup: AudioSetupValidation | null,
): SettingsSectionStatus {
  if (audioDirty) {
    return { tone: "warn", label: "Not saved" };
  }
  return formatAudioSectionStatus(audioSetup);
}

function customVoiceDirectionReady(
  config: ConfigView,
  direction: "outbound" | "inbound",
): boolean {
  const vendor = normalizeCustomVoiceVendor(
    direction === "outbound"
      ? config.outboundCustomVoiceVendor
      : config.inboundCustomVoiceVendor,
  );
  if (vendor === "fishAudio") {
    const voiceId =
      direction === "outbound"
        ? config.fishaudioVoiceId
        : config.fishaudioInboundVoiceId;
    return Boolean(config.fishaudioApiKeyConfigured) && Boolean(voiceId?.trim());
  }
  const voiceId =
    direction === "outbound"
      ? config.elevenlabsVoiceId
      : config.elevenlabsInboundVoiceId;
  return config.elevenlabsApiKeyConfigured && Boolean(voiceId?.trim());
}

export function voiceSectionStatus(
  config: ConfigView,
  voiceDirty: boolean,
): SettingsSectionStatus {
  const inboundCustom = isCustomVoiceOutput(config.inboundVoiceOutput);
  const outboundCustom = isCustomVoiceOutput(config.outboundVoiceOutput);
  if (!inboundCustom && !outboundCustom) {
    return { tone: "ok", label: "Engine voice" };
  }
  if (voiceDirty) {
    return { tone: "warn", label: "Not saved" };
  }
  const inboundReady = !inboundCustom || customVoiceDirectionReady(config, "inbound");
  const outboundReady = !outboundCustom || customVoiceDirectionReady(config, "outbound");
  if (inboundReady && outboundReady) {
    return { tone: "ok", label: "Custom voice ready" };
  }
  return { tone: "warn", label: "Setup needed" };
}

export function dirtySectionStatus(
  dirty: boolean,
): SettingsSectionStatus | undefined {
  return dirty ? { tone: "warn", label: "Not saved" } : undefined;
}

export function translateTabWarn(
  config: ConfigView,
  apiKeyDirty: boolean,
  sonioxContextDirty: boolean,
  speechDetectionDirty = false,
): boolean {
  return (
    !config.apiKeyConfigured ||
    apiKeyDirty ||
    (config.aiProvider === "soniox" && sonioxContextDirty) ||
    speechDetectionDirty
  );
}

export function intelligenceTabWarn(config: ConfigView): boolean {
  return !selectionUsable(config);
}

export function voiceTabWarn(config: ConfigView, voiceDirty: boolean): boolean {
  const inboundCustom = isCustomVoiceOutput(config.inboundVoiceOutput);
  const outboundCustom = isCustomVoiceOutput(config.outboundVoiceOutput);
  if (!inboundCustom && !outboundCustom) return false;
  return (
    voiceDirty ||
    (inboundCustom && !customVoiceDirectionReady(config, "inbound")) ||
    (outboundCustom && !customVoiceDirectionReady(config, "outbound"))
  );
}

export function audioTabWarn(
  audioDirty: boolean,
  audioSetup: AudioSetupValidation | null,
): boolean {
  return audioDirty || formatAudioSectionStatus(audioSetup).tone === "warn";
}

export function apiKeyHintBody(provider: AiProvider): string {
  if (provider === "soniox") {
    return "Create a key at soniox.com. One key covers speech translation and Soniox TTS. Stored encrypted on this device.";
  }
  if (provider === "openAi") {
    return "Create a key at platform.openai.com with Realtime access. Stored encrypted on this device.";
  }
  return "Create a key in Google AI Studio with Gemini Live access. Stored encrypted on this device.";
}

export function liveModelHintBody(
  provider: AiProvider,
  sessionMode?: "interpreter" | "notes",
): string {
  if (sessionMode === "notes" && provider === "openAi") {
    return "Notes uses OpenAI Realtime transcription (GPT Realtime Whisper). The Interpreter live model stays GPT Realtime Translate when you switch back.";
  }
  return provider === "soniox"
    ? "Soniox STT model for live translation. Meeting summaries are on the Intelligence tab."
    : "Model for live speech translation. Meeting summaries are on the Intelligence tab.";
}

export function apiKeyHintLabel(provider: AiProvider): string {
  return `About ${providerLabel(provider)} API key`;
}
