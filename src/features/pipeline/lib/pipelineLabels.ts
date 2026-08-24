import type {
  AppStatus,
  ConfigView,
  PipelineOutputMode,
} from "@/shared/lib/types/pipeline";
import { isCustomVoiceOutput } from "@/shared/lib/types/pipeline";
import { isDirectionTranslating } from "./pipelineStatus";
import { formatDirectionAudioLabel } from "@/features/audio/lib/audioSetup";

export interface PipelineModeOption {
  value: string;
  label: string;
  shortLabel?: string;
  title?: string;
  disabled?: boolean;
}

/** Outbound Translate dropdown — includes custom voice as a peer of provider translated voice. */
export type OutboundToolbarMode =
  | "translated"
  | "translatedCustom"
  | "originalAudio"
  | "textOnly";
export type InboundToolbarMode = OutboundToolbarMode;

function rawModeTooltip(aiProvider: ConfigView["aiProvider"]): string {
  const engine =
    aiProvider === "openAi"
      ? "OpenAI"
      : aiProvider === "soniox"
        ? "Soniox"
        : "Gemini";
  return `Still connected to ${engine} — uses API tokens. Use Direct for zero API usage.`;
}

const CUSTOM_VOICE_SETUP_TOOLTIP =
  "Add a custom voice API key and voice in Settings → Voice";

export function isOutboundCustomVoiceReady(config: ConfigView): boolean {
  const vendor = config.outboundCustomVoiceVendor ?? "elevenLabs";
  if (vendor === "fishAudio") {
    return (
      Boolean(config.fishaudioApiKeyConfigured) &&
      Boolean(config.fishaudioVoiceId?.trim())
    );
  }
  return (
    config.elevenlabsApiKeyConfigured && Boolean(config.elevenlabsVoiceId?.trim())
  );
}

export function isInboundCustomVoiceReady(config: ConfigView): boolean {
  const vendor = config.inboundCustomVoiceVendor ?? "elevenLabs";
  if (vendor === "fishAudio") {
    return (
      Boolean(config.fishaudioApiKeyConfigured) &&
      Boolean(config.fishaudioInboundVoiceId?.trim())
    );
  }
  return (
    config.elevenlabsApiKeyConfigured &&
    Boolean(config.elevenlabsInboundVoiceId?.trim())
  );
}

export function outboundToolbarModeFromConfig(
  config: ConfigView,
): OutboundToolbarMode {
  if (
    config.outboundMode === "translated" &&
    isCustomVoiceOutput(config.outboundVoiceOutput)
  ) {
    return "translatedCustom";
  }
  if (config.outboundMode === "originalAudio") return "originalAudio";
  if (config.outboundMode === "textOnly") return "textOnly";
  return "translated";
}

export function outboundToolbarPatch(
  mode: OutboundToolbarMode,
  config: ConfigView,
): Pick<ConfigView, "outboundMode" | "outboundVoiceOutput"> {
  switch (mode) {
    case "translatedCustom":
      return {
        outboundMode: "translated",
        outboundVoiceOutput: "custom",
      };
    case "translated":
      return {
        outboundMode: "translated",
        outboundVoiceOutput: "providerNative",
      };
    case "originalAudio":
      return {
        outboundMode: "originalAudio",
        outboundVoiceOutput: config.outboundVoiceOutput,
      };
    case "textOnly":
      return {
        outboundMode: "textOnly",
        outboundVoiceOutput: config.outboundVoiceOutput,
      };
  }
}

export function inboundToolbarModeFromConfig(
  config: ConfigView,
): InboundToolbarMode {
  if (
    config.inboundMode === "translated" &&
    isCustomVoiceOutput(config.inboundVoiceOutput)
  ) {
    return "translatedCustom";
  }
  if (config.inboundMode === "originalAudio") return "originalAudio";
  if (config.inboundMode === "textOnly") return "textOnly";
  return "translated";
}

export function inboundToolbarPatch(
  mode: InboundToolbarMode,
  config: ConfigView,
): Pick<ConfigView, "inboundMode" | "inboundVoiceOutput"> {
  switch (mode) {
    case "translatedCustom":
      return {
        inboundMode: "translated",
        inboundVoiceOutput: "custom",
      };
    case "translated":
      return {
        inboundMode: "translated",
        inboundVoiceOutput: "providerNative",
      };
    case "originalAudio":
      return {
        inboundMode: "originalAudio",
        inboundVoiceOutput: config.inboundVoiceOutput,
      };
    case "textOnly":
      return {
        inboundMode: "textOnly",
        inboundVoiceOutput: config.inboundVoiceOutput,
      };
  }
}

export function getOutboundToolbarModeOptions(
  config: ConfigView,
): PipelineModeOption[] {
  if (config.sessionMode === "notes") {
    return [
      {
        value: "originalAudio",
        label: "My voice (raw)",
        shortLabel: "Raw",
        title: "Notes mode — hear your mic; STT captions only (no translation TTS)",
      },
    ];
  }
  const customVoiceReady = isOutboundCustomVoiceReady(config);
  const translatedTitle =
    config.aiProvider === "soniox"
      ? "Soniox speech translation + Soniox TTS (same API key)"
      : "Gemini or OpenAI TTS — translated speech to the meeting";
  return [
    {
      value: "translated",
      label: "Translated voice",
      shortLabel: "Translated",
      title: translatedTitle,
    },
    {
      value: "translatedCustom",
      label: "Custom voice",
      shortLabel: "Custom",
      title: customVoiceReady
        ? "Custom voice — translated speech in your selected voice (You → Meeting)"
        : CUSTOM_VOICE_SETUP_TOOLTIP,
      disabled: !customVoiceReady,
    },
    {
      value: "originalAudio",
      label: "My voice (raw)",
      shortLabel: "Raw",
      title: rawModeTooltip(config.aiProvider),
    },
    {
      value: "textOnly",
      label: "Captions only",
      shortLabel: "Captions",
      title: "Transcript only — no audio to the meeting",
    },
  ];
}

export function formatOutboundToolbarShort(config: ConfigView): string {
  const toolbarMode = outboundToolbarModeFromConfig(config);
  const opt = getOutboundToolbarModeOptions(config).find(
    (o) => o.value === toolbarMode,
  );
  return opt?.shortLabel ?? "Translated";
}

export function getInboundToolbarModeOptions(
  config: ConfigView,
): PipelineModeOption[] {
  if (config.sessionMode === "notes") {
    return [
      {
        value: "originalAudio",
        label: "Meeting (raw)",
        shortLabel: "Raw",
        title: "Notes mode — hear meeting audio; STT captions only (no translation TTS)",
      },
    ];
  }
  const customVoiceReady = isInboundCustomVoiceReady(config);
  return [
    {
      value: "translated",
      label: "Translated audio",
      shortLabel: "Translated",
      title:
        config.aiProvider === "soniox"
          ? "Soniox speech translation + Soniox TTS"
          : "Hear translated meeting audio",
    },
    {
      value: "translatedCustom",
      label: "Custom voice",
      shortLabel: "Custom",
      title: customVoiceReady
        ? "Custom voice for translated meeting speech (Meeting → You)"
        : CUSTOM_VOICE_SETUP_TOOLTIP,
      disabled: !customVoiceReady,
    },
    {
      value: "originalAudio",
      label: "Meeting (raw)",
      shortLabel: "Raw",
      title: rawModeTooltip(config.aiProvider),
    },
    {
      value: "textOnly",
      label: "Captions only",
      shortLabel: "Captions",
      title: "Transcript only — no playback in your ears",
    },
  ];
}

export function formatInboundToolbarShort(config: ConfigView): string {
  const toolbarMode = inboundToolbarModeFromConfig(config);
  const option = getInboundToolbarModeOptions(config).find(
    (candidate) => candidate.value === toolbarMode,
  );
  return option?.shortLabel ?? "Translated";
}

export function getPipelineModeOptions(
  _direction: "inbound" = "inbound",
  config?: ConfigView,
): PipelineModeOption[] {
  if (config) return getInboundToolbarModeOptions(config);
  return [
    {
      value: "translated",
      label: "Translated audio",
      shortLabel: "Translated",
      title: "Hear translated meeting audio",
    },
    {
      value: "translatedCustom",
      label: "Custom voice",
      shortLabel: "Custom",
      title: CUSTOM_VOICE_SETUP_TOOLTIP,
      disabled: true,
    },
    {
      value: "originalAudio",
      label: "Meeting (raw)",
      shortLabel: "Raw",
      title: rawModeTooltip("gemini"),
    },
    {
      value: "textOnly",
      label: "Captions only",
      shortLabel: "Captions",
      title: "Transcript only — no playback in your ears",
    },
  ];
}

const OUTBOUND_PIPELINE_SHORT: Record<PipelineOutputMode, string> = {
  translated: "Translated",
  originalAudio: "Raw",
  textOnly: "Captions",
};

export function formatOutputModeShort(
  mode: PipelineOutputMode,
  direction: "outbound" | "inbound",
): string {
  if (direction === "outbound") {
    return OUTBOUND_PIPELINE_SHORT[mode] ?? "Translated";
  }
  const opt = getPipelineModeOptions("inbound").find((o) => o.value === mode);
  return opt?.shortLabel ?? "Translated";
}

export function formatDirectionChipLabel(
  status: AppStatus | null | undefined,
  config: ConfigView,
  direction: "outbound" | "inbound",
): string {
  const path = formatDirectionAudioLabel(status, direction);
  if (!isDirectionTranslating(status, direction)) {
    return path;
  }
  if (direction === "outbound") {
    return `${path} · ${formatOutboundToolbarShort(config)}`;
  }
  return `${path} · ${formatInboundToolbarShort(config)}`;
}
