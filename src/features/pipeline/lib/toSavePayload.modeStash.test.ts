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
});
