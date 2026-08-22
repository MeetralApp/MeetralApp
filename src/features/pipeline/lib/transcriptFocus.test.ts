import { describe, expect, it, vi } from "vitest";

import {
  dispatchTranscriptFocus,
  listenTranscriptFocus,
  TRANSCRIPT_FOCUS_EVENT,
} from "./transcriptFocus";

describe("transcriptFocus", () => {
  it("dispatches and listens for focus details", () => {
    const handler = vi.fn();
    const stop = listenTranscriptFocus(handler);
    dispatchTranscriptFocus({
      segmentId: "seg-1",
      direction: "inbound",
    });
    expect(handler).toHaveBeenCalledWith({
      segmentId: "seg-1",
      direction: "inbound",
    });
    stop();
  });

  it("ignores empty segment ids", () => {
    const handler = vi.fn();
    const stop = listenTranscriptFocus(handler);
    window.dispatchEvent(
      new CustomEvent(TRANSCRIPT_FOCUS_EVENT, {
        detail: { segmentId: "  " },
      }),
    );
    expect(handler).not.toHaveBeenCalled();
    stop();
  });
});
