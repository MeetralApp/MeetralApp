import { describe, expect, it } from "vitest";
import {
  audioDevicesDirty,
  buildSetupIssues,
  emptyDeviceRef,
  formatPipelineStartError,
  resolveGlobalAppError,
  resolveGlobalAppErrorNotice,
  resolvePipelineErrorNotice,
  formatSetupSummaryStatus,
  formatAudioSectionStatus,
  formatDirectionAudioLabel,
  formatDeviceDisplayName,
  formatRoleError,
  groupAudioDevices,
  hasRuntimeAudioFault,
  isSetupBannerVisible,
  deviceRefEquals,
  isRoleRequiredForConfig,
  isTransientDeviceUnavailable,
  isVirtualAudioDevice,
  suggestVoicemeeterBananaDevices,
  suggestBlackHoleDevices,
  suggestVirtualAudioDevices,
  type AudioSetupValidation,
  type RoleValidation,
} from "./audioSetup";
import type { AudioDeviceInfo, ConfigView } from "@/shared/lib/types/pipeline";

const baseConfig: ConfigView = {
  aiProvider: "gemini",
  apiKeyConfigured: true,
  myLanguage: "vi",
  meetingLanguage: "en",
  userMic: { id: "mic", name: "Mic" },
  teamsMicFeed: { id: "teams", name: "Teams Feed" },
  meetingCapture: { id: "cap", name: "Capture" },
  localPlayback: { id: "hp", name: "Headphones" },
  outboundMode: "translated",
  inboundMode: "translated",
  liveModel: "gemini-3.5-live-translate-preview",
  summaryModel: "gemini-2.5-flash",
  echoTargetLanguage: true,
  vadSilenceDurationMs: 800,
  vadStartSensitivity: "LOW",
  vadEndSensitivity: "LOW",
  keepDirectAudio: true,
  closeToTray: true,
  proactiveSessionRefresh: false,
  transcriptLayout: "sideBySide",
  inboundVoiceOutput: "providerNative",
  outboundVoiceOutput: "providerNative",
  elevenlabsApiKeyConfigured: false,
  elevenlabsInboundVoiceId: "",
  elevenlabsInboundTtsModel: "eleven_flash_v2_5",
  elevenlabsInboundStability: 0.5,
  elevenlabsInboundSimilarityBoost: 0.75,
  elevenlabsInboundTtsSynthesisMode: "streaming",
  elevenlabsVoiceId: "",
  elevenlabsVoices: [],
  elevenlabsModels: [],
  elevenlabsTtsModel: "eleven_flash_v2_5",
  elevenlabsStability: 0.5,
  elevenlabsSimilarityBoost: 0.75,
  elevenlabsSpeed: 1.0,
  elevenlabsUseSpeakerBoost: true,
  elevenlabsChunkSchedulePreset: "fast",
  elevenlabsTtsSynthesisMode: "streaming",
  elevenlabsPlaybackCrossfade: false,
  elevenlabsCrossfadeMs: 8,
  elevenlabsTtsLanguageAuto: true,
  elevenlabsTtsLanguageCode: "",
  sonioxTtsVoices: [],
};

const readyValidation: AudioSetupValidation = {
  outboundReady: true,
  inboundReady: true,
  roles: [
    {
      role: "userMic",
      required: true,
      configured: true,
      resolved: true,
      resolvedName: "Mic",
    },
    {
      role: "teamsMicFeed",
      required: true,
      configured: true,
      resolved: true,
      resolvedName: "Teams Feed",
    },
    {
      role: "meetingCapture",
      required: true,
      configured: true,
      resolved: true,
      resolvedName: "Capture",
    },
    {
      role: "localPlayback",
      required: true,
      configured: true,
      resolved: true,
      resolvedName: "Headphones",
    },
  ],
};

describe("emptyDeviceRef", () => {
  it("is empty by default", () => {
    expect(emptyDeviceRef()).toEqual({ id: "", name: "" });
  });
});

describe("suggestVoicemeeterBananaDevices", () => {
  const devices: AudioDeviceInfo[] = [
    {
      id: "b2",
      name: "Voicemeeter Out B2 (VB-Audio Voicemeeter VAIO)",
      direction: "input",
    },
    {
      id: "b1",
      name: "Voicemeeter Out B1 (VB-Audio Voicemeeter VAIO)",
      direction: "input",
    },
    {
      id: "aux",
      name: "Voicemeeter AUX Input (VB-Audio Voicemeeter VAIO)",
      direction: "output",
    },
    {
      id: "hp",
      name: "Headphones (Baseus)",
      direction: "output",
    },
  ];

  it("suggests B2 and AUX but not B1", () => {
    expect(suggestVoicemeeterBananaDevices(devices)).toEqual({
      meetingCapture: {
        id: "b2",
        name: "Voicemeeter Out B2 (VB-Audio Voicemeeter VAIO)",
      },
      teamsMicFeed: {
        id: "aux",
        name: "Voicemeeter AUX Input (VB-Audio Voicemeeter VAIO)",
      },
    });
  });
});

describe("suggestBlackHoleDevices", () => {
  it("maps BlackHole input and output when only 2ch is installed", () => {
    const devices: AudioDeviceInfo[] = [
      { id: "bh-in", name: "BlackHole 2ch", direction: "input" },
      { id: "bh-out", name: "BlackHole 2ch", direction: "output" },
    ];
    expect(suggestBlackHoleDevices(devices)).toEqual({
      meetingCapture: { id: "bh-in", name: "BlackHole 2ch" },
      teamsMicFeed: { id: "bh-out", name: "BlackHole 2ch" },
    });
  });

  it("prefers 16ch for meeting capture and 2ch for teams mic feed", () => {
    const devices: AudioDeviceInfo[] = [
      { id: "bh16-in", name: "BlackHole 16ch", direction: "input" },
      { id: "bh2-in", name: "BlackHole 2ch", direction: "input" },
      { id: "bh16-out", name: "BlackHole 16ch", direction: "output" },
      { id: "bh2-out", name: "BlackHole 2ch", direction: "output" },
    ];
    expect(suggestBlackHoleDevices(devices)).toEqual({
      meetingCapture: { id: "bh16-in", name: "BlackHole 16ch" },
      teamsMicFeed: { id: "bh2-out", name: "BlackHole 2ch" },
    });
  });
});

describe("suggestVirtualAudioDevices", () => {
  it("uses BlackHole on macos", () => {
    const devices: AudioDeviceInfo[] = [
      { id: "bh16", name: "BlackHole 16ch", direction: "input" },
      { id: "bh2-o", name: "BlackHole 2ch", direction: "output" },
    ];
    const result = suggestVirtualAudioDevices("macos", devices);
    expect(result.meetingCapture?.id).toBe("bh16");
    expect(result.teamsMicFeed?.id).toBe("bh2-o");
  });
});

describe("audioDevicesDirty", () => {
  it("detects draft changes", () => {
    const saved = {
      userMic: { id: "a", name: "A" },
      teamsMicFeed: { id: "b", name: "B" },
      meetingCapture: { id: "c", name: "C" },
      localPlayback: { id: "d", name: "D" },
    };
    expect(audioDevicesDirty(saved, saved)).toBe(false);
    expect(
      audioDevicesDirty(saved, {
        ...saved,
        meetingCapture: { id: "x", name: "X" },
      }),
    ).toBe(true);
  });
});

describe("isRoleRequiredForConfig", () => {
  it("skips teams mic feed for outbound text-only", () => {
    expect(
      isRoleRequiredForConfig(
        { ...baseConfig, outboundMode: "textOnly" },
        "teamsMicFeed",
        null,
      ),
    ).toBe(false);
  });

  it("requires local playback for translated inbound", () => {
    expect(
      isRoleRequiredForConfig(baseConfig, "localPlayback", null),
    ).toBe(true);
  });
});

describe("formatDeviceDisplayName", () => {
  it("strips VB-Audio suffix from Voicemeeter names", () => {
    expect(
      formatDeviceDisplayName(
        "Voicemeeter Out B2 (VB-Audio Voicemeeter VAIO)",
      ),
    ).toBe("Voicemeeter Out B2");
    expect(
      formatDeviceDisplayName(
        "Voicemeeter AUX Input (VB-Audio Voicemeeter VAIO)",
      ),
    ).toBe("Voicemeeter AUX Input");
  });

  it("truncates very long names", () => {
    const long = "A".repeat(60);
    expect(formatDeviceDisplayName(long, 20)).toHaveLength(20);
    expect(formatDeviceDisplayName(long, 20).endsWith("…")).toBe(true);
  });
});

describe("groupAudioDevices", () => {
  it("separates virtual and physical endpoints", () => {
    const devices: AudioDeviceInfo[] = [
      { id: "1", name: "Voicemeeter Out B2", direction: "input" },
      { id: "2", name: "Headphones (Baseus)", direction: "output" },
    ];
    expect(groupAudioDevices(devices)).toEqual({
      virtual: [devices[0]],
      physical: [devices[1]],
    });
    expect(isVirtualAudioDevice("VB-Audio Cable")).toBe(true);
  });
});

describe("resolveGlobalAppError", () => {
  it("shows formatted backend status.error", () => {
    expect(
      resolveGlobalAppError({
        running: true,
        outbound: "error",
        inbound: "off",
        error: "Audio device lost (microphone). Translation stopped.",
      }),
    ).toContain("microphone");
  });

  it("returns null when backend error is cleared", () => {
    expect(
      resolveGlobalAppError({
        running: true,
        outbound: "direct",
        inbound: "direct",
        error: null,
        outboundAudio: "ok",
        inboundAudio: "ok",
      }),
    ).toBeNull();
    expect(
      resolveGlobalAppError({
        running: true,
        outbound: "error",
        inbound: "off",
        error: null,
        outboundAudio: "lost",
        inboundAudio: "ok",
      }),
    ).toBeNull();
  });
});

describe("formatPipelineStartError", () => {
  it("formats raw mic device-not-found errors", () => {
    expect(
      formatPipelineStartError(
        "Your microphone: device not found (id={0.0.1.00000000}.{abc}, name=Headset (Baseus Bass BS2 Lite))",
      ),
    ).toBe(
      "“Headset (Baseus Bass BS2 Lite)” isn’t available — pick it again or use System default",
    );
  });

  it("formats meeting capture device-not-found errors", () => {
    expect(
      formatPipelineStartError(
        "Meeting capture: device not found (id=cap, name=VB-Audio Cable)",
      ),
    ).toContain("isn’t available");
  });

  it("formats bridge and configuration errors", () => {
    expect(
      formatPipelineStartError("Translation connection lost. Stop and Start again."),
    ).toContain("Connection lost");
    expect(formatPipelineStartError("Gemini API key is not configured")).toContain(
      "Settings → API",
    );
    expect(
      formatPipelineStartError("models/foo is not found for API version v1beta"),
    ).toContain("Settings → Model");
  });
});

describe("resolvePipelineErrorNotice", () => {
  it("classifies mic device-not-found as actionable device notice", () => {
    expect(
      resolvePipelineErrorNotice(
        "Your microphone: device not found (id={0.0.1.00000000}.{abc}, name=Headset (Baseus Bass BS2 Lite))",
      ),
    ).toEqual({
      kind: "device-unavailable",
      role: "userMic",
      deviceName: "Headset (Baseus Bass BS2 Lite)",
      allowSystemDefault: true,
    });
  });

  it("classifies playback device-not-found with system default", () => {
    expect(
      resolvePipelineErrorNotice(
        "Local playback: device not found (id=out, name=Headphones (Baseus Bass BS2 Lite))",
      ),
    ).toEqual({
      kind: "device-unavailable",
      role: "localPlayback",
      deviceName: "Headphones (Baseus Bass BS2 Lite)",
      allowSystemDefault: true,
    });
  });

  it("does not offer system default for meeting capture", () => {
    expect(
      resolvePipelineErrorNotice(
        "Meeting capture: device not found (id=cap, name=VB-Audio Cable)",
      ),
    ).toEqual({
      kind: "device-unavailable",
      role: "meetingCapture",
      deviceName: "VB-Audio Cable",
      allowSystemDefault: false,
    });
  });

  it("keeps fatal bridge errors as messages", () => {
    expect(
      resolvePipelineErrorNotice("Translation connection lost. Stop and Start again."),
    ).toEqual({
      kind: "fatal",
      message: "Connection lost. Stop and start the meeting to reconnect.",
    });
  });
});

describe("resolveGlobalAppErrorNotice", () => {
  it("returns structured notice from status.error", () => {
    expect(
      resolveGlobalAppErrorNotice({
        running: false,
        outbound: "error",
        inbound: "off",
        error:
          "Local playback: device not found (id=out, name=Headphones (Baseus Bass BS2 Lite))",
      }),
    ).toEqual({
      kind: "device-unavailable",
      role: "localPlayback",
      deviceName: "Headphones (Baseus Bass BS2 Lite)",
      allowSystemDefault: true,
    });
  });
});

describe("isTransientDeviceUnavailable", () => {
  it("detects configured device missing at runtime", () => {
    const entry: RoleValidation = {
      role: "userMic",
      required: true,
      configured: true,
      resolved: false,
      error:
        "Your microphone: device not found (id={0.0.1.00000000}.{abc}, name=Headset)",
    };
    expect(isTransientDeviceUnavailable(entry)).toBe(true);
  });

  it("does not treat unconfigured roles as transient", () => {
    const entry: RoleValidation = {
      role: "userMic",
      required: true,
      configured: false,
      resolved: false,
      error: "Your microphone: not configured",
    };
    expect(isTransientDeviceUnavailable(entry)).toBe(false);
  });
});

describe("isSetupBannerVisible", () => {
  const bannerInput = {
    canDirectOutbound: false,
    canDirectInbound: false,
    canTranslateOutbound: false,
    canTranslateInbound: false,
  };

  it("shows when setup issues block translation and direct", () => {
    expect(
      isSetupBannerVisible({
        ...bannerInput,
        config: { ...baseConfig, apiKeyConfigured: false },
        audioSetup: {
          ...readyValidation,
          outboundReady: false,
          inboundReady: false,
        },
      }),
    ).toBe(true);
  });

  it("hides when direct and translate are already ready", () => {
    expect(
      isSetupBannerVisible({
        config: { ...baseConfig, apiKeyConfigured: false },
        audioSetup: readyValidation,
        canDirectOutbound: true,
        canDirectInbound: true,
        canTranslateOutbound: true,
        canTranslateInbound: true,
      }),
    ).toBe(false);
  });

  it("hides api-key-only when direct path is ready", () => {
    expect(
      isSetupBannerVisible({
        config: { ...baseConfig, apiKeyConfigured: false },
        audioSetup: readyValidation,
        canDirectOutbound: true,
        canDirectInbound: false,
        canTranslateOutbound: false,
        canTranslateInbound: false,
      }),
    ).toBe(false);
  });
});

describe("buildSetupIssues", () => {
  it("ignores transient device-unavailable roles", () => {
    const validation: AudioSetupValidation = {
      outboundReady: false,
      inboundReady: true,
      roles: [
        {
          role: "userMic",
          required: true,
          configured: true,
          resolved: false,
          error: "Your microphone: device not found (id=mic, name=Mic)",
        },
        {
          role: "teamsMicFeed",
          required: true,
          configured: true,
          resolved: true,
          resolvedName: "Teams Feed",
        },
        {
          role: "meetingCapture",
          required: true,
          configured: true,
          resolved: true,
          resolvedName: "Capture",
        },
        {
          role: "localPlayback",
          required: true,
          configured: true,
          resolved: true,
          resolvedName: "Headphones",
        },
      ],
    };

    expect(buildSetupIssues(baseConfig, validation)).toEqual([]);
  });
});

describe("formatRoleError", () => {
  it("hides device GUIDs and suggests system default for mic", () => {
    expect(
      formatRoleError(
        "userMic",
        "Your microphone: device not found (id={0.0.1.00000000}.{abc}, name=Headset (Baseus Bass BS2 Lite))",
        "Headset (Baseus Bass BS2 Lite)",
      ),
    ).toBe(
      "“Headset (Baseus Bass BS2 Lite)” isn’t available — pick it again or use System default",
    );
  });

  it("uses plain copy when device name is missing", () => {
    expect(
      formatRoleError("meetingCapture", "Meeting capture: not configured"),
    ).toBe("Choose meeting capture from the list");
  });
});

describe("formatSetupSummaryStatus", () => {
  it("describes readiness state", () => {
    expect(formatSetupSummaryStatus(null)).toContain("Checking");
    expect(
      formatSetupSummaryStatus({
        outboundReady: true,
        inboundReady: true,
        roles: [],
      }),
    ).toContain("meeting and translation");
  });
});

describe("formatAudioSectionStatus", () => {
  it("returns tone and label for setup progress", () => {
    expect(formatAudioSectionStatus(null)).toEqual({
      tone: "muted",
      label: "Checking…",
    });
    expect(formatAudioSectionStatus(readyValidation)).toEqual({
      tone: "ok",
      label: "Ready",
    });
  });
});

describe("formatDirectionAudioLabel", () => {
  it("maps pipeline states to chip labels", () => {
    expect(formatDirectionAudioLabel(null, "outbound")).toBe("Off");
    expect(
      formatDirectionAudioLabel(
        { running: true, outbound: "direct", inbound: "off", error: null },
        "outbound",
      ),
    ).toBe("Direct");
    expect(
      formatDirectionAudioLabel(
        { running: true, outbound: "active", inbound: "off", error: null },
        "outbound",
      ),
    ).toBe("Translate");
    expect(
      formatDirectionAudioLabel(
        { running: true, outbound: "error", inbound: "off", error: null },
        "outbound",
      ),
    ).toBe("Error");
  });
});

describe("deviceRefEquals", () => {
  it("compares id and name", () => {
    expect(
      deviceRefEquals({ id: "a", name: "A" }, { id: "a", name: "A" }),
    ).toBe(true);
    expect(
      deviceRefEquals({ id: "a", name: "A" }, { id: "b", name: "A" }),
    ).toBe(false);
  });
});

describe("hasRuntimeAudioFault", () => {
  it("detects reconnecting or lost on either column", () => {
    expect(hasRuntimeAudioFault(null)).toBe(false);
    expect(
      hasRuntimeAudioFault({
        running: true,
        outbound: "direct",
        inbound: "off",
        error: null,
        outboundAudio: "reconnecting",
      }),
    ).toBe(true);
    expect(
      hasRuntimeAudioFault({
        running: true,
        outbound: "off",
        inbound: "active",
        error: null,
        inboundAudio: "lost",
      }),
    ).toBe(true);
  });
});
