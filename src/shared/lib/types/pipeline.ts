export type PipelineOutputMode = "translated" | "originalAudio" | "textOnly";
export type AudioPathMode = "direct" | "translate";
export type SessionMode = "interpreter" | "notes";
export type VadSensitivity = "LOW" | "MEDIUM" | "HIGH";
export type TranscriptLayout = "sideBySide" | "stacked";
export type ThemePreference = "dark" | "light" | "system";

export type OverlayPosition =
  | "bottomCenter"
  | "bottomLeft"
  | "bottomRight"
  | "topRight";

export interface OverlaySettings {
  enabled: boolean;
  opacity: number;
  position: OverlayPosition;
  offsetX: number;
  offsetY: number;
  showInbound: boolean;
  showOutbound: boolean;
  fontScale: number;
  clickThrough: boolean;
  hideFromCapture: boolean;
  autoShowWithSession: boolean;
  width: number;
  height: number;
}

export const DEFAULT_OVERLAY_SETTINGS: OverlaySettings = {
  enabled: false,
  opacity: 0.92,
  position: "bottomCenter",
  offsetX: 0,
  offsetY: 0,
  showInbound: true,
  showOutbound: true,
  fontScale: 1,
  clickThrough: false,
  hideFromCapture: true,
  autoShowWithSession: true,
  width: 420,
  height: 280,
};
export type CustomVoiceVendor = "elevenLabs" | "fishAudio" | "xai";
export type FishAudioLatency = "low" | "balanced" | "normal";
export type XaiLatency = "low" | "balanced" | "normal";
export type InboundVoiceOutput = "providerNative" | "custom";
export type OutboundVoiceOutput = "providerNative" | "custom";

export function isCustomVoiceOutput(
  value: InboundVoiceOutput | OutboundVoiceOutput | undefined,
): boolean {
  return value === "custom";
}

export function normalizeVoiceOutput<
  T extends InboundVoiceOutput | OutboundVoiceOutput,
>(value: T | undefined): T {
  return (value ?? "providerNative") as T;
}

export function normalizeCustomVoiceVendor(
  value: CustomVoiceVendor | string | undefined,
): CustomVoiceVendor {
  if (value === "xai") return "xai";
  if (value === "fishAudio") return "fishAudio";
  return "elevenLabs";
}

export interface FishAudioVoiceOption {
  voiceId: string;
  name: string;
}

export interface XaiVoiceOption {
  voiceId: string;
  name: string;
  /** `builtIn` or `custom` when known. */
  kind?: string;
}

export interface FishAudioModelOption {
  modelId: string;
  name: string;
  description?: string;
}
export type ElevenLabsChunkSchedulePreset = "live" | "fast" | "balanced" | "quality";
export type TtsSynthesisMode = "streaming" | "sentence";

export interface ElevenLabsVoiceOption {
  voiceId: string;
  name: string;
  category: string;
  previewUrl?: string;
}

export interface ElevenLabsModelOption {
  modelId: string;
  name: string;
  description?: string;
  cloneStreamSupported: boolean;
  requiresAlphaAccess: boolean;
}

export interface SonioxVoiceOption {
  id: string;
  name: string;
  gender: string;
  description?: string;
}

import type {
  AiProvider,
  LanguageInfo,
  LiveModelOption,
  SonioxTtsModelOption,
} from "@/features/ai/lib/aiTypes";

export type { AiProvider, LanguageInfo, LiveModelOption, SonioxTtsModelOption };

export interface DeviceRef {
  id: string;
  name: string;
}

export interface SonioxTranslationTerm {
  source: string;
  target: string;
}

export interface SonioxGeneralPair {
  key: string;
  value: string;
}

/** Four Soniox context sections — Always-on and each per-meeting profile. */
export interface SonioxContextPayload {
  general: SonioxGeneralPair[];
  text: string;
  terms: string[];
  translationTerms: SonioxTranslationTerm[];
}

/** App-level meeting context for summary prompts — shared 4-section shape. */
export type MeetingContextPayload = SonioxContextPayload;

export interface SonioxContextProfile {
  id: string;
  name: string;
  includeAlwaysOn: boolean;
  payload: SonioxContextPayload;
}

export function emptySonioxContextPayload(): SonioxContextPayload {
  return { general: [], text: "", terms: [], translationTerms: [] };
}

export function estimateSonioxPayloadChars(payload: SonioxContextPayload): number {
  let n = 0;
  for (const p of payload.general) {
    n += p.key.trim().length + p.value.trim().length;
  }
  n += payload.text.trim().length;
  for (const t of payload.terms) n += t.trim().length;
  for (const p of payload.translationTerms) {
    n += p.source.trim().length + p.target.trim().length;
  }
  return n;
}

/** Client-side mirror of Rust merge rules for budget preview. */
export function mergeSonioxContextPreview(
  alwaysOn: SonioxContextPayload,
  profile: SonioxContextProfile | null,
): SonioxContextPayload {
  if (!profile) return alwaysOn;
  if (!profile.includeAlwaysOn) return profile.payload;

  const general: SonioxGeneralPair[] = [];
  const seenKeys: string[] = [];
  for (const pair of profile.payload.general) {
    const key = pair.key.trim();
    if (!key || !pair.value.trim()) continue;
    if (seenKeys.some((k) => k.toLowerCase() === key.toLowerCase())) continue;
    seenKeys.push(key);
    general.push({ key, value: pair.value.trim() });
  }
  for (const pair of alwaysOn.general) {
    const key = pair.key.trim();
    if (!key || !pair.value.trim()) continue;
    if (seenKeys.some((k) => k.toLowerCase() === key.toLowerCase())) continue;
    seenKeys.push(key);
    general.push({ key, value: pair.value.trim() });
  }

  const ao = alwaysOn.text.trim();
  const mt = profile.payload.text.trim();
  const text =
    ao && mt ? `${ao}\n${mt}` : ao || mt;

  const terms: string[] = [];
  const seenTerms: string[] = [];
  for (const t of [...profile.payload.terms, ...alwaysOn.terms]) {
    const trimmed = t.trim();
    if (!trimmed) continue;
    const lower = trimmed.toLowerCase();
    if (seenTerms.includes(lower)) continue;
    seenTerms.push(lower);
    terms.push(trimmed);
  }

  const translationTerms: SonioxTranslationTerm[] = [];
  const seenSources: string[] = [];
  for (const term of [
    ...profile.payload.translationTerms,
    ...alwaysOn.translationTerms,
  ]) {
    const source = term.source.trim();
    const target = term.target.trim();
    if (!source || !target) continue;
    const key = source.toLowerCase();
    if (seenSources.includes(key)) continue;
    seenSources.push(key);
    translationTerms.push({ source, target });
  }

  return { general, text, terms, translationTerms };
}

export const SONIOX_CONTEXT_CHAR_BUDGET = 10_000;

export interface ConfigView {
  aiProvider: AiProvider;
  apiKeyConfigured: boolean;
  /** Gemini key stored (independent of active live engine). */
  geminiApiKeyConfigured?: boolean;
  /** OpenAI key stored (independent of active live engine). */
  openaiApiKeyConfigured?: boolean;
  myLanguage: string;
  meetingLanguage: string;
  /** interpreter = cross-lang translate; notes = same-lang STT capture */
  sessionMode?: SessionMode;
  /** Persisted Interpreter “You” language (restored when leaving Notes). */
  interpreterMyLanguage?: string;
  /** Persisted Interpreter “Meeting” language. */
  interpreterMeetingLanguage?: string;
  /** Persisted Notes single language. */
  notesLanguage?: string;
  interpreterOutboundMode?: PipelineOutputMode;
  interpreterInboundMode?: PipelineOutputMode;
  interpreterOutboundVoiceOutput?: OutboundVoiceOutput;
  interpreterInboundVoiceOutput?: InboundVoiceOutput;
  userMic: DeviceRef;
  teamsMicFeed: DeviceRef;
  meetingCapture: DeviceRef;
  localPlayback: DeviceRef;
  outboundMode: PipelineOutputMode;
  inboundMode: PipelineOutputMode;
  liveModel: string;
  /** Persisted live model catalogs (seeded or Soniox API). */
  geminiLiveModels?: LiveModelOption[];
  openAiLiveModels?: LiveModelOption[];
  sonioxLiveModels?: LiveModelOption[];
  summaryModel: string;
  /** Gemini or OpenAI — LLM for meeting summaries (independent of live engine). */
  summaryProvider?: AiProvider;
  echoTargetLanguage: boolean;
  vadSilenceDurationMs: number;
  vadStartSensitivity: VadSensitivity;
  vadEndSensitivity: VadSensitivity;
  keepDirectAudio: boolean;
  /** Opt-in meeting Opus recording (default false). */
  saveMeetingAudio?: boolean;
  /** Absolute folder for recordings; empty = app default. */
  saveMeetingAudioFolder?: string;
  inboundOriginalUnderTranslation?: boolean;
  inboundOriginalDuckedGain?: number;
  closeToTray: boolean;
  themePreference?: ThemePreference;
  proactiveSessionRefresh: boolean;
  transcriptLayout: TranscriptLayout;
  overlay?: OverlaySettings;
  inboundVoiceOutput?: InboundVoiceOutput;
  outboundVoiceOutput: OutboundVoiceOutput;
  outboundCustomVoiceVendor?: CustomVoiceVendor;
  inboundCustomVoiceVendor?: CustomVoiceVendor;
  sonioxApiKeyConfigured?: boolean;
  sonioxAlwaysOn?: SonioxContextPayload;
  sonioxContextProfiles?: SonioxContextProfile[];
  /** null / undefined = Always-on only */
  sonioxActiveContextProfileId?: string | null;
  sonioxTtsVoice?: string;
  /** You → Meeting Soniox TTS voice when Engine voice. Defaults from inbound voice. */
  sonioxTtsOutboundVoice?: string;
  sonioxTtsModel?: string;
  /** Cached Soniox TTS voice catalog (persisted; Refresh re-syncs). */
  sonioxTtsVoices?: SonioxVoiceOption[];
  /** Cached Soniox TTS model catalog (persisted; Refresh re-syncs). */
  sonioxTtsModels?: SonioxTtsModelOption[];
  /** Meeting → You Soniox TTS speed (0.7–1.3). Default 1.0. */
  sonioxTtsInboundSpeed?: number;
  /** You → Meeting Soniox TTS speed when Engine voice (0.7–1.3). Default 1.0. */
  sonioxTtsOutboundSpeed?: number;
  /** Soniox STT endpoint detection (0–3). Default 2. */
  sonioxEndpointLatencyAdjustmentLevel?: number;
  /** Soniox STT endpoint sensitivity (−1–1). Default 0.3. */
  sonioxEndpointSensitivity?: number;
  /** Soniox STT max endpoint delay ms (500–3000). Default 1000. */
  sonioxMaxEndpointDelayMs?: number;
  summaryFallbackAvailable?: boolean;
  elevenlabsApiKeyConfigured: boolean;
  fishaudioApiKeyConfigured?: boolean;
  xaiApiKeyConfigured?: boolean;
  elevenlabsInboundVoiceId?: string;
  elevenlabsInboundTtsModel?: string;
  elevenlabsInboundStability?: number;
  elevenlabsInboundSimilarityBoost?: number;
  elevenlabsInboundTtsSynthesisMode?: TtsSynthesisMode;
  elevenlabsVoiceId: string;
  /** Cached ElevenLabs My Voices list (persisted; Refresh re-syncs). */
  elevenlabsVoices?: ElevenLabsVoiceOption[];
  /** Cached ElevenLabs TTS models (persisted; Refresh re-syncs). */
  elevenlabsModels?: ElevenLabsModelOption[];
  elevenlabsTtsModel: string;
  elevenlabsStability: number;
  elevenlabsSimilarityBoost: number;
  elevenlabsSpeed: number;
  elevenlabsUseSpeakerBoost: boolean;
  elevenlabsChunkSchedulePreset: ElevenLabsChunkSchedulePreset;
  elevenlabsTtsLanguageAuto: boolean;
  elevenlabsTtsLanguageCode: string;
  elevenlabsTtsSynthesisMode: TtsSynthesisMode;
  elevenlabsPlaybackCrossfade: boolean;
  elevenlabsCrossfadeMs: number;
  fishaudioVoiceId?: string;
  fishaudioInboundVoiceId?: string;
  fishaudioVoices?: FishAudioVoiceOption[];
  /** Cached Fish Audio TTS models (persisted; Refresh re-syncs allow-list). */
  fishaudioModels?: FishAudioModelOption[];
  fishaudioTtsModel?: string;
  fishaudioInboundTtsModel?: string;
  fishaudioLatency?: FishAudioLatency;
  fishaudioInboundLatency?: FishAudioLatency;
  fishaudioTemperature?: number;
  fishaudioInboundTemperature?: number;
  fishaudioSpeed?: number;
  fishaudioTopP?: number;
  xaiVoiceId?: string;
  xaiInboundVoiceId?: string;
  xaiVoices?: XaiVoiceOption[];
  xaiLatency?: XaiLatency;
  xaiInboundLatency?: XaiLatency;
  xaiSpeed?: number;
  /** Meeting Intelligence. */
  artifactsEnabled?: boolean;
  /** Preferred AI output language; "" = match the meeting's You language. */
  answerLanguage?: string;
  /** App-level meeting context injected into summary prompts. */
  meetingContext?: MeetingContextPayload;
  /** Custom OpenAI-compatible LLM profiles. Keys never leave the backend. */
  customLlmProfiles?: CustomLlmProfileView[];
  /** Selected custom profile; null/undefined = built-in summaryProvider. */
  summaryCustomProfileId?: string | null;
}

/** Custom OpenAI-compatible LLM profile as exposed to the FE (no API key). */
export interface CustomLlmProfileView {
  id: string;
  label: string;
  baseUrl: string;
  chatModel: string;
  /** False when the save probe found `response_format` unsupported (prompt-only JSON). */
  jsonMode?: boolean;
  apiKeyConfigured: boolean;
}

/** Input for upsert/test custom LLM profile commands. */
export interface CustomLlmProfileInput {
  /** Omitted/empty = create; set = edit in place. */
  id?: string;
  label: string;
  baseUrl: string;
  chatModel: string;
  /** undefined = keep existing (edit) / none (create); "" = clear; value = set. */
  apiKey?: string;
}

/** Returned by `save_config` when live audio paths were hot-rewired. */
export interface SaveConfigResult {
  rewired?: boolean;
}

export interface SaveConfigPayload {
  aiProvider: AiProvider;
  geminiApiKey: string;
  clearGeminiApiKey?: boolean;
  openaiApiKey: string;
  clearOpenaiApiKey?: boolean;
  sonioxApiKey?: string;
  clearSonioxApiKey?: boolean;
  myLanguage: string;
  meetingLanguage: string;
  sessionMode?: SessionMode;
  interpreterMyLanguage?: string;
  interpreterMeetingLanguage?: string;
  notesLanguage?: string;
  interpreterOutboundMode?: PipelineOutputMode;
  interpreterInboundMode?: PipelineOutputMode;
  interpreterOutboundVoiceOutput?: OutboundVoiceOutput;
  interpreterInboundVoiceOutput?: InboundVoiceOutput;
  userMic: DeviceRef;
  teamsMicFeed: DeviceRef;
  meetingCapture: DeviceRef;
  localPlayback: DeviceRef;
  outboundMode: PipelineOutputMode;
  inboundMode: PipelineOutputMode;
  liveModel: string;
  geminiLiveModels?: LiveModelOption[];
  openAiLiveModels?: LiveModelOption[];
  sonioxLiveModels?: LiveModelOption[];
  summaryModel: string;
  summaryProvider?: AiProvider;
  echoTargetLanguage: boolean;
  vadSilenceDurationMs: number;
  vadStartSensitivity: VadSensitivity;
  vadEndSensitivity: VadSensitivity;
  keepDirectAudio: boolean;
  saveMeetingAudio?: boolean;
  saveMeetingAudioFolder?: string;
  inboundOriginalUnderTranslation?: boolean;
  inboundOriginalDuckedGain?: number;
  closeToTray: boolean;
  themePreference?: ThemePreference;
  proactiveSessionRefresh: boolean;
  transcriptLayout: TranscriptLayout;
  overlay?: OverlaySettings;
  inboundVoiceOutput: InboundVoiceOutput;
  outboundVoiceOutput: OutboundVoiceOutput;
  outboundCustomVoiceVendor?: CustomVoiceVendor;
  inboundCustomVoiceVendor?: CustomVoiceVendor;
  sonioxAlwaysOn?: SonioxContextPayload;
  sonioxContextProfiles?: SonioxContextProfile[];
  /** Pass `""` or `null` to clear active profile (Always-on only). */
  sonioxActiveContextProfileId?: string | null;
  sonioxTtsVoice?: string;
  sonioxTtsOutboundVoice?: string;
  sonioxTtsModel?: string;
  sonioxTtsVoices?: SonioxVoiceOption[];
  sonioxTtsModels?: SonioxTtsModelOption[];
  sonioxTtsInboundSpeed?: number;
  sonioxTtsOutboundSpeed?: number;
  sonioxEndpointLatencyAdjustmentLevel?: number;
  sonioxEndpointSensitivity?: number;
  sonioxMaxEndpointDelayMs?: number;
  elevenlabsApiKey: string;
  clearElevenlabsApiKey?: boolean;
  elevenlabsInboundVoiceId: string;
  elevenlabsInboundTtsModel?: string;
  elevenlabsInboundStability?: number;
  elevenlabsInboundSimilarityBoost?: number;
  elevenlabsInboundTtsSynthesisMode?: TtsSynthesisMode;
  elevenlabsVoiceId: string;
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
  elevenlabsPlaybackCrossfade?: boolean;
  elevenlabsCrossfadeMs?: number;
  fishaudioApiKey?: string;
  clearFishaudioApiKey?: boolean;
  fishaudioVoiceId?: string;
  fishaudioInboundVoiceId?: string;
  fishaudioVoices?: FishAudioVoiceOption[];
  /** Cached Fish Audio TTS models (persisted; Refresh re-syncs allow-list). */
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
  /** Meeting Intelligence. */
  artifactsEnabled?: boolean;
  answerLanguage?: string;
  /** App-level meeting context for summary prompts. */
  meetingContext?: MeetingContextPayload;
  /** Selected custom LLM profile; "" clears to built-in. */
  summaryCustomProfileId?: string | null;
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  direction: string;
}

export interface DevicesResponse {
  devices: AudioDeviceInfo[];
}

export type PipelineState =
  | "off"
  | "direct"
  | "starting"
  | "stopping"
  | "active"
  | "error";

export type BridgeConnectionState = "idle" | "ready" | "reconnecting";

export type AudioConnectionState = "ok" | "reconnecting" | "lost";

export interface AppStatus {
  running: boolean;
  outbound: PipelineState;
  inbound: PipelineState;
  error: string | null;
  outboundActiveSince?: number | null;
  inboundActiveSince?: number | null;
  outboundBridge?: BridgeConnectionState;
  inboundBridge?: BridgeConnectionState;
  outboundReconnectAttempt?: number | null;
  inboundReconnectAttempt?: number | null;
  outboundAudio?: AudioConnectionState;
  inboundAudio?: AudioConnectionState;
  outboundAudioReconnectAttempt?: number | null;
  inboundAudioReconnectAttempt?: number | null;
  micMuted?: boolean;
  speakerMuted?: boolean;
}

export interface TranscriptEvent {
  direction: string;
  sourceText?: string | null;
  translatedText?: string | null;
  interim: boolean;
  turnComplete?: boolean;
  inputSegmentFinished?: boolean;
  outputSegmentFinished?: boolean;
  connectionGap?: boolean;
  liveSource?: string | null;
  liveTranslated?: string | null;
}
