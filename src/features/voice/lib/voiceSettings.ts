import type {
  ConfigView,
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
  { value: "elevenLabsClone", label: "ElevenLabs voice" },
];

export function engineVoiceHint(aiProvider: ConfigView["aiProvider"]): string {
  if (aiProvider === "soniox") {
    return "Soniox speech translation plus Soniox TTS (same API key).";
  }
  return "Audio from the live Gemini or OpenAI session.";
}

export function inboundVoiceNote(aiProvider: ConfigView["aiProvider"]): string {
  if (aiProvider === "soniox") {
    return "Engine voice uses Soniox TTS, or pick an ElevenLabs voice.";
  }
  return "Built-in live session voice, or an ElevenLabs voice.";
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
