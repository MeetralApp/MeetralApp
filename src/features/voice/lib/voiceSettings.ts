import type {
  CustomVoiceVendor,
  ConfigView,
  FishAudioLatency,
  FishAudioModelOption,
  InboundVoiceOutput,
  OutboundVoiceOutput,
  SonioxVoiceOption,
  TtsSynthesisMode,
} from "@/shared/lib/types/pipeline";

export const OUTPUT_OPTIONS: {
  value: InboundVoiceOutput | OutboundVoiceOutput;
  label: string;
}[] = [
  { value: "providerNative", label: "Engine voice" },
  { value: "custom", label: "Custom voice" },
];

export const CUSTOM_VOICE_VENDOR_OPTIONS: { value: CustomVoiceVendor; label: string }[] = [
  { value: "elevenLabs", label: "ElevenLabs" },
  { value: "fishAudio", label: "Fish Audio" },
];

export const FISH_LATENCY_OPTIONS: {
  value: FishAudioLatency;
  label: string;
  hint: string;
}[] = [
  {
    value: "low",
    label: "Low",
    hint: "Fastest first audio. Slightly less stable prosody.",
  },
  {
    value: "balanced",
    label: "Balanced",
    hint: "Recommended default for live meetings.",
  },
  {
    value: "normal",
    label: "Normal",
    hint: "Highest quality, extra latency.",
  },
];

export const FALLBACK_FISH_MODELS: FishAudioModelOption[] = [
  { modelId: "s2.1-pro", name: "S2.1 Pro" },
  { modelId: "s2.1-pro-free", name: "S2.1 Pro (free, no latency SLA)" },
  { modelId: "s2-pro", name: "S2 Pro" },
  { modelId: "s1", name: "S1" },
];

export function engineVoiceHint(aiProvider: ConfigView["aiProvider"]): string {
  if (aiProvider === "soniox") {
    return "Soniox speech translation plus Soniox TTS (same API key).";
  }
  return "Audio from the live Gemini or OpenAI session.";
}

export function inboundVoiceNote(aiProvider: ConfigView["aiProvider"]): string {
  if (aiProvider === "soniox") {
    return "Engine voice uses Soniox TTS, or pick a custom voice.";
  }
  return "Built-in live session voice, or a custom voice.";
}

export const FALLBACK_SONIOX_VOICES: SonioxVoiceOption[] = [
  { id: "Adrian", name: "Adrian", gender: "male" },
  { id: "Maya", name: "Maya", gender: "female" },
  { id: "Noah", name: "Noah", gender: "male" },
  { id: "Emma", name: "Emma", gender: "female" },
  { id: "Claire", name: "Claire", gender: "female" },
];

export const SYNTHESIS_MODE_OPTIONS: {
  value: TtsSynthesisMode;
  label: string;
  hint: string;
}[] = [
  {
    value: "streaming",
    label: "Speed",
    hint: "Streams translation as it arrives and flushes at each sentence end. Lowest latency — best for live meetings.",
  },
  {
    value: "sentence",
    label: "Natural",
    hint: "Waits for each full sentence before speaking. Smoothest tone, with about one sentence of extra delay.",
  },
];
