import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  AudioPathMode,
  InboundVoiceOutput,
  OutboundVoiceOutput,
  PipelineOutputMode,
} from "@/shared/lib/types/pipeline";
import type { AppSnapshot, DeviceCatalogState } from "@/features/pipeline/lib/appState";
import { normalizeAppStatus } from "@/features/pipeline/lib/pipelineStatus";

export async function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}

export async function getStatus(): Promise<AppStatus> {
  return normalizeAppStatus(await invoke<AppStatus>("get_status"));
}

export async function refreshDeviceCatalog(): Promise<DeviceCatalogState> {
  return invoke<DeviceCatalogState>("refresh_device_catalog");
}

export async function setOutboundOutputMode(
  mode: PipelineOutputMode,
): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_outbound_output_mode", { mode }),
  );
}

export async function setOutboundVoiceOutput(
  voiceOutput: OutboundVoiceOutput,
): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_outbound_voice_output", { voiceOutput }),
  );
}

export async function setInboundOutputMode(
  mode: PipelineOutputMode,
): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_inbound_output_mode", { mode }),
  );
}

export async function setInboundVoiceOutput(
  voiceOutput: InboundVoiceOutput,
): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_inbound_voice_output", { voiceOutput }),
  );
}

export async function setOutboundAudioMode(
  mode: AudioPathMode,
): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_outbound_audio_mode", { mode }),
  );
}

export async function setInboundAudioMode(
  mode: AudioPathMode,
): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_inbound_audio_mode", { mode }),
  );
}

export async function ensureDirectAudio(): Promise<AppStatus> {
  return normalizeAppStatus(await invoke<AppStatus>("ensure_direct_audio"));
}

export async function setMicMuted(muted: boolean): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_mic_muted", { muted }),
  );
}

export async function setSpeakerMuted(muted: boolean): Promise<AppStatus> {
  return normalizeAppStatus(
    await invoke<AppStatus>("set_speaker_muted", { muted }),
  );
}

/** Hot-preview Quiet volume / underlay while Settings draft is dirty (no disk write). */
export async function previewInboundDucking(args: {
  enabled: boolean;
  gain: number;
}): Promise<void> {
  await invoke("preview_inbound_ducking", {
    request: {
      enabled: args.enabled,
      gain: args.gain,
    },
  });
}
