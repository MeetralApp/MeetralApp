import type { AudioSetupValidation } from "@/features/audio/lib/audioSetup";
import type {
  AppStatus,
  AudioDeviceInfo,
  ConfigView,
  PipelineState,
} from "@/shared/lib/types/pipeline";
import { DEFAULT_OVERLAY_SETTINGS } from "@/shared/lib/types/pipeline";
import { normalizePipelineOutputMode } from "./pipelineStatus";

export interface DeviceCatalogState {
  revision: number;
  devices: AudioDeviceInfo[];
  enumeratedAt: number;
}

export interface ColumnIdleBadgeSnapshot {
  kind: string;
  label: string;
  title: string;
}

export interface ColumnUiState {
  pipeline: PipelineState;
  audioConnection: import("@/features/audio/lib/audioConnection").AudioConnectionState;
  canDirect: boolean;
  canTranslate: boolean;
  translateDisabledReason: string | null;
  idleBadge: ColumnIdleBadgeSnapshot;
  pipelineLive: boolean;
  muteEnabled: boolean;
}

export interface ColumnUiSnapshot {
  outbound: ColumnUiState;
  inbound: ColumnUiState;
}

export interface SetupState {
  revision: number;
  apiKeyConfigured: boolean;
  canDirectOutbound: boolean;
  canDirectInbound: boolean;
  canTranslateOutbound: boolean;
  canTranslateInbound: boolean;
  outboundIdleBadge: ColumnIdleBadgeSnapshot;
  inboundIdleBadge: ColumnIdleBadgeSnapshot;
  audio: AudioSetupValidation;
}

export interface ConfigState extends ConfigView {
  revision: number;
}

export interface AppSnapshot {
  revision: number;
  runtime: AppStatus;
  setup: SetupState;
  columns: ColumnUiSnapshot;
  devices: DeviceCatalogState;
  config: ConfigState;
}

export function applyRevision<T extends { revision: number }>(
  previous: T | null,
  next: T,
): T {
  if (previous && next.revision < previous.revision) {
    return previous;
  }
  return next;
}

export function normalizeConfigView(data: ConfigView): ConfigView {
  return {
    ...data,
    aiProvider: data.aiProvider ?? "gemini",
    outboundMode: normalizePipelineOutputMode(data.outboundMode),
    inboundMode: normalizePipelineOutputMode(data.inboundMode),
    keepDirectAudio: data.keepDirectAudio ?? true,
    saveMeetingAudio: data.saveMeetingAudio ?? false,
    saveMeetingAudioFolder: data.saveMeetingAudioFolder ?? "",
    inboundOriginalUnderTranslation: data.inboundOriginalUnderTranslation ?? true,
    inboundOriginalDuckedGain: data.inboundOriginalDuckedGain ?? 0.18,
    closeToTray: data.closeToTray ?? true,
    themePreference: data.themePreference ?? "dark",
    proactiveSessionRefresh: data.proactiveSessionRefresh ?? false,
    transcriptLayout: data.transcriptLayout ?? "sideBySide",
    overlay: { ...DEFAULT_OVERLAY_SETTINGS, ...(data.overlay ?? {}) },
    inboundVoiceOutput: data.inboundVoiceOutput ?? "providerNative",
    outboundVoiceOutput: data.outboundVoiceOutput ?? "providerNative",
    outboundCustomVoiceVendor: data.outboundCustomVoiceVendor ?? "elevenLabs",
    inboundCustomVoiceVendor: data.inboundCustomVoiceVendor ?? "elevenLabs",
    sonioxAlwaysOn: data.sonioxAlwaysOn ?? {
      general: [],
      text: "",
      terms: [],
      translationTerms: [],
    },
    sonioxContextProfiles: data.sonioxContextProfiles ?? [],
    sonioxActiveContextProfileId: data.sonioxActiveContextProfileId ?? null,
    sonioxTtsVoice: data.sonioxTtsVoice ?? "Adrian",
    sonioxTtsOutboundVoice:
      data.sonioxTtsOutboundVoice ?? data.sonioxTtsVoice ?? "Adrian",
    sonioxTtsModel: data.sonioxTtsModel ?? "tts-rt-v1",
    sonioxTtsVoices: data.sonioxTtsVoices ?? [],
    sonioxTtsModels: data.sonioxTtsModels ?? [],
    sonioxTtsInboundSpeed: data.sonioxTtsInboundSpeed ?? 1.0,
    sonioxTtsOutboundSpeed: data.sonioxTtsOutboundSpeed ?? 1.0,
    sonioxEndpointLatencyAdjustmentLevel:
      data.sonioxEndpointLatencyAdjustmentLevel ?? 2,
    sonioxEndpointSensitivity: data.sonioxEndpointSensitivity ?? 0.3,
    sonioxMaxEndpointDelayMs: data.sonioxMaxEndpointDelayMs ?? 1000,
    elevenlabsApiKeyConfigured: data.elevenlabsApiKeyConfigured ?? false,
    elevenlabsInboundVoiceId: data.elevenlabsInboundVoiceId ?? "",
    elevenlabsInboundTtsModel:
      data.elevenlabsInboundTtsModel ?? "eleven_flash_v2_5",
    elevenlabsInboundStability: data.elevenlabsInboundStability ?? 0.5,
    elevenlabsInboundSimilarityBoost:
      data.elevenlabsInboundSimilarityBoost ?? 0.75,
    elevenlabsInboundTtsSynthesisMode:
      data.elevenlabsInboundTtsSynthesisMode ?? "streaming",
    elevenlabsVoiceId: data.elevenlabsVoiceId ?? "",
    elevenlabsVoices: data.elevenlabsVoices ?? [],
    elevenlabsModels: data.elevenlabsModels ?? [],
    elevenlabsTtsModel: data.elevenlabsTtsModel ?? "eleven_flash_v2_5",
    elevenlabsStability: data.elevenlabsStability ?? 0.5,
    elevenlabsSimilarityBoost: data.elevenlabsSimilarityBoost ?? 0.75,
    elevenlabsSpeed: data.elevenlabsSpeed ?? 1.0,
    elevenlabsUseSpeakerBoost: data.elevenlabsUseSpeakerBoost ?? true,
    elevenlabsChunkSchedulePreset: data.elevenlabsChunkSchedulePreset ?? "live",
    elevenlabsTtsLanguageAuto: data.elevenlabsTtsLanguageAuto ?? true,
    elevenlabsTtsLanguageCode: data.elevenlabsTtsLanguageCode ?? "",
    elevenlabsTtsSynthesisMode: data.elevenlabsTtsSynthesisMode ?? "streaming",
    elevenlabsPlaybackCrossfade: data.elevenlabsPlaybackCrossfade ?? false,
    elevenlabsCrossfadeMs: data.elevenlabsCrossfadeMs ?? 8,
    fishaudioVoiceId: data.fishaudioVoiceId ?? "",
    fishaudioInboundVoiceId: data.fishaudioInboundVoiceId ?? "",
    fishaudioVoices: data.fishaudioVoices ?? [],
    fishaudioModels: data.fishaudioModels ?? [],
    fishaudioTtsModel: data.fishaudioTtsModel ?? "s2.1-pro",
    fishaudioInboundTtsModel: data.fishaudioInboundTtsModel ?? "s2.1-pro",
    fishaudioLatency: data.fishaudioLatency ?? "balanced",
    fishaudioInboundLatency: data.fishaudioInboundLatency ?? "balanced",
    fishaudioTemperature: data.fishaudioTemperature ?? 0.7,
    fishaudioInboundTemperature: data.fishaudioInboundTemperature ?? 0.7,
    fishaudioSpeed: data.fishaudioSpeed ?? 1.0,
    fishaudioTopP: data.fishaudioTopP ?? 0.7,
    xaiApiKeyConfigured: data.xaiApiKeyConfigured ?? false,
    xaiVoiceId: data.xaiVoiceId ?? "eve",
    xaiInboundVoiceId: data.xaiInboundVoiceId ?? "eve",
    xaiVoices: data.xaiVoices ?? [],
    xaiLatency: data.xaiLatency ?? "balanced",
    xaiInboundLatency: data.xaiInboundLatency ?? "balanced",
    xaiSpeed: data.xaiSpeed ?? 1.0,
    liveModel:
      data.liveModel ??
      (data.aiProvider === "openAi"
        ? "gpt-realtime-translate"
        : data.aiProvider === "soniox"
          ? "stt-rt-v5"
          : "gemini-3.5-live-translate-preview"),
    geminiLiveModels: data.geminiLiveModels ?? [],
    openAiLiveModels: data.openAiLiveModels ?? [],
    sonioxLiveModels: data.sonioxLiveModels ?? [],
    summaryModel:
      data.summaryModel ??
      (data.summaryProvider === "openAi" || data.aiProvider === "openAi"
        ? "gpt-4o"
        : "gemini-2.5-flash"),
    summaryProvider:
      data.summaryProvider === "openAi"
        ? "openAi"
        : data.summaryProvider === "gemini"
          ? "gemini"
          : data.aiProvider === "openAi"
            ? "openAi"
            : "gemini",
    customLlmProfiles: data.customLlmProfiles ?? [],
    summaryCustomProfileId: data.summaryCustomProfileId ?? null,
  };
}

export function configViewFromState(state: ConfigState): ConfigView {
  const { revision: _revision, ...config } = state;
  return normalizeConfigView(config);
}

export function hasSetupIssuesFromSetup(setup: SetupState | null): boolean {
  if (!setup) return true;
  if (!setup.apiKeyConfigured) return true;
  return !setup.canDirectOutbound && !setup.canDirectInbound;
}

export function formatSetupAttentionFromSetup(
  setup: SetupState | null,
): string {
  if (!setup) return "Settings — setup needed";
  const apiMissing = !setup.apiKeyConfigured;
  const audioMissing = !setup.canDirectOutbound && !setup.canDirectInbound;

  if (apiMissing && audioMissing) {
    return "Settings — API key and audio setup needed";
  }
  if (apiMissing) return "Settings — API key missing";
  return "Settings — audio setup incomplete";
}
