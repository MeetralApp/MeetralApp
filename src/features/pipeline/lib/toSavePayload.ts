import type {
  ConfigView,
  CustomVoiceVendor,
  ElevenLabsChunkSchedulePreset,
  FishAudioLatency,
  FishAudioModelOption,
  FishAudioVoiceOption,
  InboundVoiceOutput,
  SaveConfigPayload,
  OutboundVoiceOutput,
  SonioxContextPayload,
  SonioxContextProfile,
  SonioxVoiceOption,
  ElevenLabsModelOption,
  ElevenLabsVoiceOption,
  LiveModelOption,
  MeetingContextPayload,
  SonioxTtsModelOption,
  TtsSynthesisMode,
  XaiLatency,
  XaiVoiceOption,
} from "@/shared/lib/types/pipeline";
import { DEFAULT_OVERLAY_SETTINGS } from "@/shared/lib/types/pipeline";

export function toSavePayload(
  config: ConfigView,
  options: {
    geminiApiKey?: string;
    clearGeminiApiKey?: boolean;
    openaiApiKey?: string;
    clearOpenaiApiKey?: boolean;
    sonioxApiKey?: string;
    clearSonioxApiKey?: boolean;
    inboundVoiceOutput?: InboundVoiceOutput;
    outboundVoiceOutput?: OutboundVoiceOutput;
    outboundCustomVoiceVendor?: CustomVoiceVendor;
    inboundCustomVoiceVendor?: CustomVoiceVendor;
    sonioxAlwaysOn?: SonioxContextPayload;
    sonioxContextProfiles?: SonioxContextProfile[];
    sonioxActiveContextProfileId?: string | null;
    sonioxTtsVoice?: string;
    sonioxTtsOutboundVoice?: string;
    sonioxTtsModel?: string;
    /** Catalog-only persist — do not send a selected model (keeps the user's pick). */
    skipSonioxTtsModel?: boolean;
    sonioxTtsVoices?: SonioxVoiceOption[];
    sonioxTtsModels?: SonioxTtsModelOption[];
    sonioxTtsInboundSpeed?: number;
    sonioxTtsOutboundSpeed?: number;
    sonioxEndpointLatencyAdjustmentLevel?: number;
    sonioxEndpointSensitivity?: number;
    sonioxMaxEndpointDelayMs?: number;
    geminiLiveModels?: LiveModelOption[];
    openAiLiveModels?: LiveModelOption[];
    sonioxLiveModels?: LiveModelOption[];
    liveModel?: string;
    elevenlabsApiKey?: string;
    clearElevenlabsApiKey?: boolean;
    elevenlabsInboundVoiceId?: string;
    elevenlabsInboundTtsModel?: string;
    elevenlabsInboundStability?: number;
    elevenlabsInboundSimilarityBoost?: number;
    elevenlabsInboundTtsSynthesisMode?: TtsSynthesisMode;
    elevenlabsVoiceId?: string;
    elevenlabsVoices?: ElevenLabsVoiceOption[];
    elevenlabsModels?: ElevenLabsModelOption[];
    elevenlabsTtsModel?: string;
    elevenlabsStability?: number;
    elevenlabsSimilarityBoost?: number;
    elevenlabsSpeed?: number;
    elevenlabsUseSpeakerBoost?: boolean;
    elevenlabsChunkSchedulePreset?: ElevenLabsChunkSchedulePreset;
    elevenlabsTtsLanguageAuto?: boolean;
    elevenlabsTtsLanguageCode?: string;
    elevenlabsTtsSynthesisMode?: TtsSynthesisMode;
    fishaudioApiKey?: string;
    clearFishaudioApiKey?: boolean;
    fishaudioVoiceId?: string;
    fishaudioInboundVoiceId?: string;
    fishaudioVoices?: FishAudioVoiceOption[];
    fishaudioModels?: FishAudioModelOption[];
    fishaudioTtsModel?: string;
    fishaudioInboundTtsModel?: string;
    fishaudioLatency?: FishAudioLatency;
    fishaudioInboundLatency?: FishAudioLatency;
    fishaudioTemperature?: number;
    fishaudioInboundTemperature?: number;
    fishaudioSpeed?: number;
    fishaudioTopP?: number;
    xaiApiKey?: string;
    clearXaiApiKey?: boolean;
    xaiVoiceId?: string;
    xaiInboundVoiceId?: string;
    xaiVoices?: XaiVoiceOption[];
    xaiLatency?: XaiLatency;
    xaiInboundLatency?: XaiLatency;
    xaiSpeed?: number;
    artifactsEnabled?: boolean;
    answerLanguage?: string;
    meetingContext?: MeetingContextPayload;
  } = {},
): SaveConfigPayload {
  const notesMode = (config.sessionMode ?? "interpreter") === "notes";
  const outboundVoiceOutput =
    options.outboundVoiceOutput ??
    config.outboundVoiceOutput ??
    "providerNative";
  const inboundVoiceOutput =
    options.inboundVoiceOutput ??
    config.inboundVoiceOutput ??
    "providerNative";
  return {
    aiProvider: config.aiProvider,
    geminiApiKey: options.geminiApiKey ?? "",
    clearGeminiApiKey: options.clearGeminiApiKey,
    openaiApiKey: options.openaiApiKey ?? "",
    clearOpenaiApiKey: options.clearOpenaiApiKey,
    sonioxApiKey: options.sonioxApiKey ?? "",
    clearSonioxApiKey: options.clearSonioxApiKey,
    myLanguage: config.myLanguage,
    meetingLanguage: config.meetingLanguage,
    sessionMode: config.sessionMode ?? "interpreter",
    // Stashes: prefer explicit stash fields; mirror active pipeline prefs while in Interpreter.
    interpreterMyLanguage: config.interpreterMyLanguage ?? config.myLanguage,
    interpreterMeetingLanguage:
      config.interpreterMeetingLanguage ?? config.meetingLanguage,
    notesLanguage: config.notesLanguage ?? config.myLanguage,
    interpreterOutboundMode: notesMode
      ? (config.interpreterOutboundMode ?? "translated")
      : config.outboundMode,
    interpreterInboundMode: notesMode
      ? (config.interpreterInboundMode ?? "translated")
      : config.inboundMode,
    interpreterOutboundVoiceOutput: notesMode
      ? (config.interpreterOutboundVoiceOutput ?? outboundVoiceOutput)
      : outboundVoiceOutput,
    interpreterInboundVoiceOutput: notesMode
      ? (config.interpreterInboundVoiceOutput ?? inboundVoiceOutput)
      : inboundVoiceOutput,
    userMic: config.userMic,
    teamsMicFeed: config.teamsMicFeed,
    meetingCapture: config.meetingCapture,
    localPlayback: config.localPlayback,
    outboundMode: config.outboundMode,
    inboundMode: config.inboundMode,
    liveModel: options.liveModel ?? config.liveModel,
    geminiLiveModels: options.geminiLiveModels ?? config.geminiLiveModels ?? [],
    openAiLiveModels: options.openAiLiveModels ?? config.openAiLiveModels ?? [],
    sonioxLiveModels: options.sonioxLiveModels ?? config.sonioxLiveModels ?? [],
    summaryModel: config.summaryModel,
    summaryProvider:
      config.summaryProvider === "openAi" ? "openAi" : "gemini",
    // "" = built-in; id = custom profile (Rust treats absent as unchanged).
    summaryCustomProfileId: config.summaryCustomProfileId ?? "",
    echoTargetLanguage: config.echoTargetLanguage,
    vadSilenceDurationMs: config.vadSilenceDurationMs,
    vadStartSensitivity: config.vadStartSensitivity,
    vadEndSensitivity: config.vadEndSensitivity,
    keepDirectAudio: config.keepDirectAudio,
    saveMeetingAudio: config.saveMeetingAudio ?? false,
    saveMeetingAudioFolder: config.saveMeetingAudioFolder ?? "",
    inboundOriginalUnderTranslation: config.inboundOriginalUnderTranslation ?? true,
    inboundOriginalDuckedGain: config.inboundOriginalDuckedGain ?? 0.18,
    closeToTray: config.closeToTray,
    themePreference: config.themePreference ?? "dark",
    proactiveSessionRefresh: config.proactiveSessionRefresh,
    transcriptLayout: config.transcriptLayout,
    overlay: config.overlay ?? { ...DEFAULT_OVERLAY_SETTINGS },
    inboundVoiceOutput,
    outboundVoiceOutput,
    outboundCustomVoiceVendor:
      options.outboundCustomVoiceVendor ??
      config.outboundCustomVoiceVendor ??
      "elevenLabs",
    inboundCustomVoiceVendor:
      options.inboundCustomVoiceVendor ??
      config.inboundCustomVoiceVendor ??
      "elevenLabs",
    sonioxAlwaysOn:
      options.sonioxAlwaysOn ??
      config.sonioxAlwaysOn ?? {
        general: [],
        text: "",
        terms: [],
        translationTerms: [],
      },
    sonioxContextProfiles:
      options.sonioxContextProfiles ?? config.sonioxContextProfiles ?? [],
    sonioxActiveContextProfileId:
      options.sonioxActiveContextProfileId !== undefined
        ? (options.sonioxActiveContextProfileId ?? "")
        : (config.sonioxActiveContextProfileId ?? ""),
    sonioxTtsVoice: options.sonioxTtsVoice ?? config.sonioxTtsVoice ?? "Adrian",
    sonioxTtsOutboundVoice:
      options.sonioxTtsOutboundVoice ??
      config.sonioxTtsOutboundVoice ??
      config.sonioxTtsVoice ??
      "Adrian",
    sonioxTtsModel: options.skipSonioxTtsModel
      ? undefined
      : (options.sonioxTtsModel ?? config.sonioxTtsModel ?? "tts-rt-v1"),
    sonioxTtsVoices: options.sonioxTtsVoices ?? config.sonioxTtsVoices ?? [],
    sonioxTtsModels: options.sonioxTtsModels ?? config.sonioxTtsModels ?? [],
    sonioxTtsInboundSpeed:
      options.sonioxTtsInboundSpeed ?? config.sonioxTtsInboundSpeed ?? 1.0,
    sonioxTtsOutboundSpeed:
      options.sonioxTtsOutboundSpeed ?? config.sonioxTtsOutboundSpeed ?? 1.0,
    sonioxEndpointLatencyAdjustmentLevel:
      options.sonioxEndpointLatencyAdjustmentLevel ??
      config.sonioxEndpointLatencyAdjustmentLevel ??
      2,
    sonioxEndpointSensitivity:
      options.sonioxEndpointSensitivity ??
      config.sonioxEndpointSensitivity ??
      0.3,
    sonioxMaxEndpointDelayMs:
      options.sonioxMaxEndpointDelayMs ?? config.sonioxMaxEndpointDelayMs ?? 1000,
    elevenlabsApiKey: options.elevenlabsApiKey ?? "",
    clearElevenlabsApiKey: options.clearElevenlabsApiKey,
    elevenlabsInboundVoiceId:
      options.elevenlabsInboundVoiceId ?? config.elevenlabsInboundVoiceId ?? "",
    elevenlabsInboundTtsModel:
      options.elevenlabsInboundTtsModel ??
      config.elevenlabsInboundTtsModel ??
      "eleven_flash_v2_5",
    elevenlabsInboundStability:
      options.elevenlabsInboundStability ??
      config.elevenlabsInboundStability ??
      0.5,
    elevenlabsInboundSimilarityBoost:
      options.elevenlabsInboundSimilarityBoost ??
      config.elevenlabsInboundSimilarityBoost ??
      0.75,
    elevenlabsInboundTtsSynthesisMode:
      options.elevenlabsInboundTtsSynthesisMode ??
      config.elevenlabsInboundTtsSynthesisMode ??
      "streaming",
    elevenlabsVoiceId: options.elevenlabsVoiceId ?? config.elevenlabsVoiceId ?? "",
    elevenlabsVoices: options.elevenlabsVoices ?? config.elevenlabsVoices ?? [],
    elevenlabsModels: options.elevenlabsModels ?? config.elevenlabsModels ?? [],
    elevenlabsTtsModel:
      options.elevenlabsTtsModel ?? config.elevenlabsTtsModel ?? "eleven_flash_v2_5",
    elevenlabsStability:
      options.elevenlabsStability ?? config.elevenlabsStability ?? 0.5,
    elevenlabsSimilarityBoost:
      options.elevenlabsSimilarityBoost ?? config.elevenlabsSimilarityBoost ?? 0.75,
    elevenlabsSpeed: options.elevenlabsSpeed ?? config.elevenlabsSpeed ?? 1.0,
    elevenlabsUseSpeakerBoost:
      options.elevenlabsUseSpeakerBoost ?? config.elevenlabsUseSpeakerBoost ?? true,
    elevenlabsChunkSchedulePreset:
      options.elevenlabsChunkSchedulePreset ??
      config.elevenlabsChunkSchedulePreset ??
      "live",
    elevenlabsTtsLanguageAuto:
      options.elevenlabsTtsLanguageAuto ?? config.elevenlabsTtsLanguageAuto ?? true,
    elevenlabsTtsLanguageCode:
      options.elevenlabsTtsLanguageCode ?? config.elevenlabsTtsLanguageCode ?? "",
    elevenlabsTtsSynthesisMode:
      options.elevenlabsTtsSynthesisMode ??
      config.elevenlabsTtsSynthesisMode ??
      "streaming",
    fishaudioApiKey: options.fishaudioApiKey ?? "",
    clearFishaudioApiKey: options.clearFishaudioApiKey,
    fishaudioVoiceId:
      options.fishaudioVoiceId ?? config.fishaudioVoiceId ?? "",
    fishaudioInboundVoiceId:
      options.fishaudioInboundVoiceId ?? config.fishaudioInboundVoiceId ?? "",
    fishaudioVoices: options.fishaudioVoices ?? config.fishaudioVoices ?? [],
    fishaudioModels: options.fishaudioModels ?? config.fishaudioModels ?? [],
    fishaudioTtsModel:
      options.fishaudioTtsModel ?? config.fishaudioTtsModel ?? "s2.1-pro",
    fishaudioInboundTtsModel:
      options.fishaudioInboundTtsModel ??
      config.fishaudioInboundTtsModel ??
      "s2.1-pro",
    fishaudioLatency:
      options.fishaudioLatency ?? config.fishaudioLatency ?? "balanced",
    fishaudioInboundLatency:
      options.fishaudioInboundLatency ??
      config.fishaudioInboundLatency ??
      "balanced",
    fishaudioTemperature:
      options.fishaudioTemperature ?? config.fishaudioTemperature ?? 0.7,
    fishaudioInboundTemperature:
      options.fishaudioInboundTemperature ??
      config.fishaudioInboundTemperature ??
      0.7,
    fishaudioSpeed: options.fishaudioSpeed ?? config.fishaudioSpeed ?? 1.0,
    fishaudioTopP: options.fishaudioTopP ?? config.fishaudioTopP ?? 0.7,
    xaiApiKey: options.xaiApiKey ?? "",
    clearXaiApiKey: options.clearXaiApiKey,
    xaiVoiceId: options.xaiVoiceId ?? config.xaiVoiceId ?? "eve",
    xaiInboundVoiceId:
      options.xaiInboundVoiceId ?? config.xaiInboundVoiceId ?? "eve",
    xaiVoices: options.xaiVoices ?? config.xaiVoices ?? [],
    xaiLatency: options.xaiLatency ?? config.xaiLatency ?? "balanced",
    xaiInboundLatency:
      options.xaiInboundLatency ?? config.xaiInboundLatency ?? "balanced",
    xaiSpeed: options.xaiSpeed ?? config.xaiSpeed ?? 1.0,
    artifactsEnabled:
      options.artifactsEnabled ?? config.artifactsEnabled ?? true,
    answerLanguage: options.answerLanguage ?? config.answerLanguage ?? "",
    // Only emit meetingContext when the user explicitly edited it.
    // Absent → backend None = keep existing; a fabricated empty payload
    // would silently wipe the user's configured summary domain/glossary.
    ...(options.meetingContext !== undefined
      ? { meetingContext: options.meetingContext }
      : {}),
  };
}
