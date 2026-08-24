import { describe, expect, it } from "vitest";

import { baseConfig } from "@/test/fixtures/config";
import { toSavePayload } from "./toSavePayload";

describe("toSavePayload mode stashes", () => {
  it("preserves interpreter stash while session is notes", () => {
    const payload = toSavePayload({
      ...baseConfig,
      sessionMode: "notes",
      myLanguage: "ja",
      meetingLanguage: "ja",
      notesLanguage: "ja",
      interpreterMyLanguage: "vi",
      interpreterMeetingLanguage: "en",
      interpreterOutboundMode: "textOnly",
      interpreterInboundMode: "translated",
      outboundMode: "originalAudio",
      inboundMode: "originalAudio",
    });

    expect(payload.sessionMode).toBe("notes");
    expect(payload.notesLanguage).toBe("ja");
    expect(payload.interpreterMyLanguage).toBe("vi");
    expect(payload.interpreterMeetingLanguage).toBe("en");
    expect(payload.interpreterOutboundMode).toBe("textOnly");
    expect(payload.interpreterInboundMode).toBe("translated");
  });

  it("mirrors active pipeline modes into interpreter stash while interpreter", () => {
    const payload = toSavePayload({
      ...baseConfig,
      sessionMode: "interpreter",
      outboundMode: "textOnly",
      inboundMode: "originalAudio",
      interpreterOutboundMode: "translated",
      interpreterInboundMode: "translated",
    });

    expect(payload.interpreterOutboundMode).toBe("textOnly");
    expect(payload.interpreterInboundMode).toBe("originalAudio");
  });

  it("mirrors voice-output options into interpreter stash so Engine/Custom is not reverted", () => {
    const payload = toSavePayload(
      {
        ...baseConfig,
        sessionMode: "interpreter",
        outboundVoiceOutput: "custom",
        inboundVoiceOutput: "custom",
        interpreterOutboundVoiceOutput: "custom",
        interpreterInboundVoiceOutput: "custom",
      },
      {
        outboundVoiceOutput: "providerNative",
        inboundVoiceOutput: "providerNative",
      },
    );
    expect(payload.outboundVoiceOutput).toBe("providerNative");
    expect(payload.inboundVoiceOutput).toBe("providerNative");
    expect(payload.interpreterOutboundVoiceOutput).toBe("providerNative");
    expect(payload.interpreterInboundVoiceOutput).toBe("providerNative");
  });

  it("keeps interpreter voice stash while session is notes", () => {
    const payload = toSavePayload(
      {
        ...baseConfig,
        sessionMode: "notes",
        outboundVoiceOutput: "providerNative",
        inboundVoiceOutput: "providerNative",
        interpreterOutboundVoiceOutput: "custom",
        interpreterInboundVoiceOutput: "custom",
      },
      { outboundVoiceOutput: "providerNative" },
    );
    expect(payload.interpreterOutboundVoiceOutput).toBe("custom");
    expect(payload.interpreterInboundVoiceOutput).toBe("custom");
  });

  it("omits selected Soniox TTS model when persisting catalogs only", () => {
    const payload = toSavePayload(
      { ...baseConfig, sonioxTtsModel: "tts-rt-v1" },
      {
        skipSonioxTtsModel: true,
        sonioxTtsModels: [{ id: "tts-rt-v1", languages: [] }],
      },
    );
    expect(payload.sonioxTtsModel).toBeUndefined();
    expect(payload.sonioxTtsModels).toHaveLength(1);
  });
});
