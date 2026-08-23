import type {
  AudioDeviceInfo,
  AppStatus,
  ConfigView,
  PipelineOutputMode,
} from "@/shared/lib/types/pipeline";
import { isCustomVoiceOutput } from "@/shared/lib/types/pipeline";
import { needsPlaybackMode } from "@/features/pipeline/lib/pipelineStatus";
import { providerLabel } from "@/features/ai/lib/aiTypes";

export type AudioRole =
  | "userMic"
  | "teamsMicFeed"
  | "meetingCapture"
  | "localPlayback";

export interface DeviceRef {
  id: string;
  name: string;
}

export interface RoleValidation {
  role: AudioRole;
  required: boolean;
  configured: boolean;
  resolved: boolean;
  resolvedName?: string | null;
  error?: string | null;
}

export interface AudioSetupValidation {
  outboundReady: boolean;
  inboundReady: boolean;
  roles: RoleValidation[];
}

export const AUDIO_ROLE_LABELS: Record<AudioRole, string> = {
  userMic: "Your mic",
  teamsMicFeed: "To meeting",
  meetingCapture: "From meeting",
  localPlayback: "Playback",
};

/** Short noun phrases for error / choose copy (not the UI field title). */
export const AUDIO_ROLE_CHOOSE: Record<AudioRole, string> = {
  userMic: "your mic",
  teamsMicFeed: "the to-meeting device",
  meetingCapture: "meeting capture",
  localPlayback: "playback",
};

export const AUDIO_ROLE_HINTS: Record<AudioRole, string> = {
  userMic: "What you speak into.",
  teamsMicFeed: "Sent to the meeting as your mic.",
  meetingCapture: "Captured from the meeting.",
  localPlayback: "Where translation plays.",
};

export const AUDIO_ROLE_ORDER: AudioRole[] = [
  "userMic",
  "teamsMicFeed",
  "meetingCapture",
  "localPlayback",
];

export type AudioDevicesDraft = Pick<
  ConfigView,
  "userMic" | "teamsMicFeed" | "meetingCapture" | "localPlayback"
>;

export const AUDIO_ROLE_FIELDS: {
  role: AudioRole;
  field: keyof AudioDevicesDraft;
  direction: "input" | "output";
  allowSystemDefault: boolean;
}[] = [
  {
    role: "userMic",
    field: "userMic",
    direction: "input",
    allowSystemDefault: true,
  },
  {
    role: "meetingCapture",
    field: "meetingCapture",
    direction: "input",
    allowSystemDefault: false,
  },
  {
    role: "teamsMicFeed",
    field: "teamsMicFeed",
    direction: "output",
    allowSystemDefault: false,
  },
  {
    role: "localPlayback",
    field: "localPlayback",
    direction: "output",
    allowSystemDefault: true,
  },
];

export function emptyDeviceRef(): DeviceRef {
  return { id: "", name: "" };
}

export function roleFromValidation(
  validation: AudioSetupValidation | null | undefined,
  role: AudioRole,
): RoleValidation | undefined {
  return validation?.roles.find((r) => r.role === role);
}

/** Device was configured but is temporarily missing from the OS (hot-unplug). */
export function isTransientDeviceUnavailable(
  entry: RoleValidation | undefined,
): boolean {
  if (!entry?.required || entry.resolved) return false;
  if (!entry.configured) return false;
  return /device not found/i.test(entry.error ?? "");
}

const BACKEND_ROLE_LABEL_PREFIX: Record<AudioRole, string> = {
  userMic: "Your microphone",
  teamsMicFeed: "Teams mic feed",
  meetingCapture: "Meeting capture",
  localPlayback: "Local playback",
};

/** Short Live-banner titles for missing devices. */
export const AUDIO_ROLE_UNAVAILABLE_TITLES: Record<AudioRole, string> = {
  userMic: "Mic unavailable",
  teamsMicFeed: "Meeting mic unavailable",
  meetingCapture: "Meeting capture unavailable",
  localPlayback: "Playback unavailable",
};

export type PipelineErrorNotice =
  | {
      kind: "device-unavailable";
      role: AudioRole | null;
      deviceName: string | null;
      allowSystemDefault: boolean;
    }
  | {
      kind: "fatal";
      message: string;
    };

function extractDeviceNameFromError(error: string): string | undefined {
  const marker = "name=";
  const idx = error.indexOf(marker);
  if (idx === -1) return undefined;
  const start = idx + marker.length;
  const end = error.lastIndexOf(")");
  if (end <= start) return undefined;
  const name = error.slice(start, end).trim();
  return name || undefined;
}

function isDeviceUnavailableError(error: string): boolean {
  return (
    /device not found/i.test(error) ||
    /id=\{/.test(error) ||
    /id=0\./.test(error)
  );
}

/** Classify a raw/backend error for the Live global banner. */
export function resolvePipelineErrorNotice(
  error: string | null | undefined,
): PipelineErrorNotice | null {
  if (!error?.trim()) return null;
  const raw = error.replace(/^Error:\s*/i, "").trim();

  for (const role of AUDIO_ROLE_ORDER) {
    const prefix = BACKEND_ROLE_LABEL_PREFIX[role];
    if (!raw.startsWith(prefix)) continue;
    if (/not configured/i.test(raw)) {
      return { kind: "fatal", message: formatRoleError(role, raw) };
    }
    if (isDeviceUnavailableError(raw)) {
      return {
        kind: "device-unavailable",
        role,
        deviceName: extractDeviceNameFromError(raw) ?? null,
        allowSystemDefault: roleAllowsSystemDefault(role),
      };
    }
    return { kind: "fatal", message: formatRoleError(role, raw) };
  }

  if (/device not found/i.test(raw)) {
    return {
      kind: "device-unavailable",
      role: null,
      deviceName: extractDeviceNameFromError(raw) ?? null,
      allowSystemDefault: false,
    };
  }

  return { kind: "fatal", message: formatPipelineStartError(raw) };
}

/** User-facing copy for pipeline start failures (backend returns raw role errors). */
export function formatPipelineStartError(error: string): string {
  const raw = error.replace(/^Error:\s*/i, "").trim();
  if (
    /translation connection lost|bridge fatal|websocket|ws closed/i.test(raw)
  ) {
    return "Connection lost. Stop and start the meeting to reconnect.";
  }
  if (/api key is not configured|missing api key|gemini api key/i.test(raw)) {
    return "Add your API key in Settings → API.";
  }
  if (
    /is not found for API version|is not supported for generateContent|model not found/i.test(
      raw,
    )
  ) {
    return "Selected model unavailable. Pick another in Settings → Model.";
  }
  for (const role of AUDIO_ROLE_ORDER) {
    const prefix = BACKEND_ROLE_LABEL_PREFIX[role];
    if (raw.startsWith(prefix)) {
      return formatRoleError(role, raw);
    }
  }
  if (/device not found/i.test(raw)) {
    return "Audio device isn’t available — check the connection or pick the device again in Settings";
  }
  return raw;
}

/** Single global error line — mirrors backend status.error only. */
export function resolveGlobalAppError(
  status: AppStatus | null | undefined,
): string | null {
  if (status?.error) {
    return formatPipelineStartError(status.error);
  }
  return null;
}

/** Structured Live banner notice — mirrors backend status.error only. */
export function resolveGlobalAppErrorNotice(
  status: AppStatus | null | undefined,
): PipelineErrorNotice | null {
  return resolvePipelineErrorNotice(status?.error);
}

export function formatSetupSummaryStatus(
  validation: AudioSetupValidation | null | undefined,
): string {
  if (!validation) return "Checking audio devices…";
  const { outboundReady, inboundReady } = validation;
  if (outboundReady && inboundReady) {
    return "Audio ready for meeting and translation";
  }
  if (outboundReady) return "Your voice path is ready — finish meeting → you devices";
  if (inboundReady) return "Meeting audio path is ready — finish you → meeting devices";

  const missingCount = validation.roles.filter(
    (entry) =>
      entry.required &&
      !entry.resolved &&
      !isTransientDeviceUnavailable(entry),
  ).length;
  if (missingCount === 1) return "One device still needs to be set up";
  if (missingCount > 1) return `${missingCount} devices still need to be set up`;
  return "Finish audio setup below";
}

export function formatAudioSectionStatus(
  validation: AudioSetupValidation | null | undefined,
): { tone: "ok" | "warn" | "muted"; label: string } {
  if (!validation) {
    return { tone: "muted", label: "Checking…" };
  }
  if (validation.outboundReady && validation.inboundReady) {
    return { tone: "ok", label: "Ready" };
  }
  const missingCount = validation.roles.filter(
    (entry) =>
      entry.required &&
      !entry.resolved &&
      !isTransientDeviceUnavailable(entry),
  ).length;
  if (missingCount === 1) {
    return { tone: "warn", label: "1 to set up" };
  }
  if (missingCount > 1) {
    return { tone: "warn", label: `${missingCount} to set up` };
  }
  return { tone: "warn", label: "Setup needed" };
}

function roleAllowsSystemDefault(role: AudioRole): boolean {
  return role === "userMic" || role === "localPlayback";
}

/** Turn backend validation errors into short, user-facing copy (no GUIDs). */
export function formatRoleError(
  role: AudioRole,
  error: string | null | undefined,
  savedDeviceName?: string | null,
): string {
  const label = AUDIO_ROLE_LABELS[role];
  const choose = AUDIO_ROLE_CHOOSE[role];

  if (!error) {
    return roleAllowsSystemDefault(role)
      ? `Choose ${choose} or use System default`
      : `Choose ${choose} from the list`;
  }

  const deviceName =
    savedDeviceName?.trim() || extractDeviceNameFromError(error);

  if (/not configured/i.test(error)) {
    return roleAllowsSystemDefault(role)
      ? `Choose ${choose} or use System default`
      : `Choose ${choose} from the list`;
  }

  if (/device not found/i.test(error)) {
    if (deviceName) {
      const short = formatDeviceDisplayName(deviceName, 36);
      return roleAllowsSystemDefault(role)
        ? `“${short}” isn’t available — pick it again or use System default`
        : `“${short}” isn’t available — pick it again from the list`;
    }
    return roleAllowsSystemDefault(role)
      ? `${label} isn’t available — pick a device or use System default`
      : `${label} isn’t available — pick a device from the list`;
  }

  if (/id=\{/.test(error) || /id=0\./.test(error)) {
    return roleAllowsSystemDefault(role)
      ? `${label} isn’t available — pick a device or use System default`
      : `${label} isn’t available — pick a device from the list`;
  }

  return `${label} needs attention — choose a device below`;
}

function savedDeviceNameForRole(
  config: ConfigView,
  role: AudioRole,
): string | undefined {
  switch (role) {
    case "userMic":
      return config.userMic.name || undefined;
    case "teamsMicFeed":
      return config.teamsMicFeed.name || undefined;
    case "meetingCapture":
      return config.meetingCapture.name || undefined;
    case "localPlayback":
      return config.localPlayback.name || undefined;
  }
}

export function buildSetupIssues(
  config: ConfigView,
  validation: AudioSetupValidation | null | undefined,
): string[] {
  const issues: string[] = [];

  if (!config.apiKeyConfigured) {
    issues.push(
      `Add a ${providerLabel(config.aiProvider)} API key to enable translation`,
    );
  }

  if (isCustomVoiceOutput(config.outboundVoiceOutput)) {
    const vendor = config.outboundCustomVoiceVendor ?? "elevenLabs";
    if (vendor === "fishAudio") {
      if (!config.fishaudioApiKeyConfigured) {
        issues.push("Add your Fish Audio API key in Settings → Voice for custom voice");
      }
      if (!config.fishaudioVoiceId?.trim()) {
        issues.push("Select a Fish Audio voice in Settings → Voice");
      }
    } else {
      if (!config.elevenlabsApiKeyConfigured) {
        issues.push("Add your ElevenLabs API key in Settings → Voice for custom voice");
      }
      if (!config.elevenlabsVoiceId?.trim()) {
        issues.push("Select or enter an ElevenLabs voice in Settings → Voice");
      }
    }
  }

  if (isCustomVoiceOutput(config.inboundVoiceOutput)) {
    const vendor = config.inboundCustomVoiceVendor ?? "elevenLabs";
    if (vendor === "fishAudio") {
      if (!config.fishaudioApiKeyConfigured) {
        issues.push(
          "Add your Fish Audio API key in Settings → Voice for Meeting custom voice",
        );
      }
      if (!config.fishaudioInboundVoiceId?.trim()) {
        issues.push("Select a Fish Audio voice for Meeting → You in Settings → Voice");
      }
    } else if (!config.elevenlabsApiKeyConfigured) {
      issues.push(
        "Add your ElevenLabs API key in Settings → Voice for Meeting custom voice",
      );
    } else if (!config.elevenlabsInboundVoiceId?.trim()) {
      issues.push("Select an ElevenLabs voice for Meeting → You in Settings → Voice");
    }
  }

  for (const role of [
    "userMic",
    "teamsMicFeed",
    "meetingCapture",
    "localPlayback",
  ] as AudioRole[]) {
    const entry = roleFromValidation(validation, role);
    if (!entry?.required || entry.resolved) continue;
    if (isTransientDeviceUnavailable(entry)) continue;
    issues.push(
      formatRoleError(role, entry.error, savedDeviceNameForRole(config, role)),
    );
  }

  if (
    issues.length === 0 &&
    validation &&
    !validation.outboundReady &&
    !validation.inboundReady
  ) {
    const hasRealSetupGap = validation.roles.some(
      (entry) =>
        entry.required &&
        !entry.resolved &&
        !isTransientDeviceUnavailable(entry),
    );
    if (hasRealSetupGap) {
      issues.push("Check your audio device selections below");
    }
  }

  return issues;
}

export function hasRuntimeAudioFault(
  status: import("@/shared/lib/types/pipeline").AppStatus | null | undefined,
): boolean {
  if (!status) return false;
  return (
    status.outboundAudio === "reconnecting" ||
    status.outboundAudio === "lost" ||
    status.inboundAudio === "reconnecting" ||
    status.inboundAudio === "lost"
  );
}

/**
* Whether the Live global SetupBanner should render.
* When true, column toolbar chips for `setup` / `api-key` stay hidden
* so the strip is the single setup signal.
*/
export function isSetupBannerVisible(input: {
  config: ConfigView;
  status?: AppStatus | null;
  audioSetup: AudioSetupValidation | null;
  canDirectOutbound: boolean;
  canDirectInbound: boolean;
  canTranslateOutbound: boolean;
  canTranslateInbound: boolean;
}): boolean {
  const {
    config,
    status = null,
    audioSetup,
    canDirectOutbound,
    canDirectInbound,
    canTranslateOutbound,
    canTranslateInbound,
  } = input;

  if (hasRuntimeAudioFault(status)) return false;

  const directReady = canDirectOutbound || canDirectInbound;
  const translateReady = canTranslateOutbound || canTranslateInbound;
  if (directReady && translateReady) return false;

  const issues = buildSetupIssues(config, audioSetup);
  if (issues.length === 0) return false;

  const apiKeyOnly =
    !config.apiKeyConfigured &&
    issues.every((issue) => /api key/i.test(issue));
  if (directReady && apiKeyOnly) return false;

  return true;
}

export function formatDirectionAudioLabel(
  status: import("@/shared/lib/types/pipeline").AppStatus | null | undefined,
  direction: "outbound" | "inbound",
): string {
  if (!status) return "Off";
  const state = direction === "outbound" ? status.outbound : status.inbound;
  if (state === "direct") return "Direct";
  if (state === "starting" || state === "stopping" || state === "active") return "Translate";
  if (state === "error") return "Error";
  return "Off";
}

export function isRoleRequiredForConfig(
  config: ConfigView,
  role: AudioRole,
  validation?: AudioSetupValidation | null,
): boolean {
  const entry = roleFromValidation(validation, role);
  if (entry) return entry.required;
  switch (role) {
    case "userMic":
      return true;
    case "teamsMicFeed":
      return needsPlaybackMode(config.outboundMode);
    case "meetingCapture":
      return true;
    case "localPlayback":
      return needsPlaybackMode(config.inboundMode);
  }
}

export function deviceRefEquals(a: DeviceRef, b: DeviceRef): boolean {
  return a.id === b.id && a.name === b.name;
}

export function audioDevicesDirty(
  saved: AudioDevicesDraft,
  draft: AudioDevicesDraft,
): boolean {
  return (
    !deviceRefEquals(saved.userMic, draft.userMic) ||
    !deviceRefEquals(saved.teamsMicFeed, draft.teamsMicFeed) ||
    !deviceRefEquals(saved.meetingCapture, draft.meetingCapture) ||
    !deviceRefEquals(saved.localPlayback, draft.localPlayback)
  );
}

export function isVirtualAudioDevice(name: string): boolean {
  return /voicemeeter|vb-audio|virtual|cable|vb cable|blackhole|loopback|soundflower/i.test(
    name,
  );
}

/** Short label for dropdowns — keeps full name in title attribute. */
export function formatDeviceDisplayName(name: string, maxLen = 52): string {
  if (!name.trim()) return "";
  const stripped = name
    .replace(/\s*\(VB-Audio[^)]*\)\s*/gi, "")
    .replace(/\s{2,}/g, " ")
    .trim();
  const display = stripped || name;
  if (display.length <= maxLen) return display;
  return `${display.slice(0, maxLen - 1)}…`;
}

export function groupAudioDevices(devices: AudioDeviceInfo[]): {
  virtual: AudioDeviceInfo[];
  physical: AudioDeviceInfo[];
} {
  const virtual: AudioDeviceInfo[] = [];
  const physical: AudioDeviceInfo[] = [];
  for (const device of devices) {
    if (isVirtualAudioDevice(device.name)) {
      virtual.push(device);
    } else {
      physical.push(device);
    }
  }
  return { virtual, physical };
}

function findDeviceByPatterns(
  devices: AudioDeviceInfo[],
  direction: "input" | "output",
  patterns: RegExp[],
): DeviceRef | null {
  const filtered = devices.filter((d) => d.direction === direction);
  for (const pattern of patterns) {
    const match = filtered.find((d) => pattern.test(d.name));
    if (match) {
      return { id: match.id, name: match.name };
    }
  }
  return null;
}

export function suggestVoicemeeterBananaDevices(
  devices: AudioDeviceInfo[],
): Partial<Pick<AudioDevicesDraft, "teamsMicFeed" | "meetingCapture">> {
  const meetingCapture = findDeviceByPatterns(devices, "input", [
    /voicemeeter out b2/i,
    /out b2.*voicemeeter/i,
  ]);
  const teamsMicFeed = findDeviceByPatterns(devices, "output", [
    /voicemeeter aux input/i,
    /aux input.*voicemeeter/i,
  ]);

  return {
    ...(meetingCapture ? { meetingCapture } : {}),
    ...(teamsMicFeed ? { teamsMicFeed } : {}),
  };
}

export function suggestBlackHoleDevices(
  devices: AudioDeviceInfo[],
): Partial<Pick<AudioDevicesDraft, "teamsMicFeed" | "meetingCapture">> {
  const meetingCapture = findDeviceByPatterns(devices, "input", [
    /blackhole\s*16/i,
    /16\s*ch.*blackhole/i,
    /blackhole.*16\s*ch/i,
    /blackhole\s*2/i,
    /2\s*ch.*blackhole/i,
    /blackhole.*2\s*ch/i,
    /blackhole/i,
  ]);
  const teamsMicFeed = findDeviceByPatterns(devices, "output", [
    /blackhole\s*2/i,
    /2\s*ch.*blackhole/i,
    /blackhole.*2\s*ch/i,
    /blackhole\s*16/i,
    /16\s*ch.*blackhole/i,
    /blackhole.*16\s*ch/i,
    /blackhole/i,
  ]);

  return {
    ...(meetingCapture ? { meetingCapture } : {}),
    ...(teamsMicFeed ? { teamsMicFeed } : {}),
  };
}

export type SupportedPlatform = "windows" | "macos" | string;

export function suggestVirtualAudioDevices(
  platform: SupportedPlatform,
  devices: AudioDeviceInfo[],
): Partial<Pick<AudioDevicesDraft, "teamsMicFeed" | "meetingCapture">> {
  if (platform === "macos") {
    return suggestBlackHoleDevices(devices);
  }
  return suggestVoicemeeterBananaDevices(devices);
}

export type { PipelineOutputMode };
