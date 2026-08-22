import { describe, expect, it } from "vitest";

import { formatAudioBytes } from "./formatAudioBytes";

describe("formatAudioBytes", () => {
  it("formats zero and sub-MB as KB", () => {
    expect(formatAudioBytes(0)).toBe("0 MB");
    expect(formatAudioBytes(512)).toBe("0.5 KB");
    expect(formatAudioBytes(10 * 1024)).toBe("10 KB");
  });

  it("formats MB and GB", () => {
    expect(formatAudioBytes(1.5 * 1024 * 1024)).toBe("1.5 MB");
    expect(formatAudioBytes(42 * 1024 * 1024)).toBe("42 MB");
    expect(formatAudioBytes(1.25 * 1024 * 1024 * 1024)).toBe("1.3 GB");
    expect(formatAudioBytes(12.4 * 1024 * 1024 * 1024)).toBe("12.4 GB");
  });
});
