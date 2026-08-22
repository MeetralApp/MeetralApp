import { describe, expect, it } from "vitest";

import {
  escapeRegExp,
  queryTokens,
  segmentMatchesQuery,
  splitHighlightParts,
} from "./highlightSnippet";

describe("escapeRegExp", () => {
  it("escapes regex metacharacters", () => {
    expect(escapeRegExp("a+b?")).toBe("a\\+b\\?");
  });
});

describe("queryTokens", () => {
  it("splits and dedupes case-insensitively", () => {
    expect(queryTokens("  Advocate advocate STATE ")).toEqual([
      "Advocate",
      "STATE",
    ]);
  });

  it("drops 1-char tokens when longer ones exist", () => {
    expect(queryTokens("a budget")).toEqual(["budget"]);
  });

  it("keeps a single 1-char token", () => {
    expect(queryTokens("a")).toEqual(["a"]);
  });
});

describe("segmentMatchesQuery", () => {
  it("matches source or translation tokens", () => {
    expect(
      segmentMatchesQuery(
        { sourceText: "Approve the budget", translatedText: "Phê duyệt" },
        "budget",
      ),
    ).toBe(true);
    expect(
      segmentMatchesQuery(
        { sourceText: "Hello", translatedText: "Phê duyệt ngân sách" },
        "ngân",
      ),
    ).toBe(true);
    expect(
      segmentMatchesQuery(
        { sourceText: "Hello", translatedText: "Xin chào" },
        "budget",
      ),
    ).toBe(false);
  });
});

describe("splitHighlightParts", () => {
  it("returns plain text when query empty", () => {
    expect(splitHighlightParts("hello", "")).toEqual([
      { text: "hello", match: false },
    ]);
  });

  it("highlights matching tokens", () => {
    expect(splitHighlightParts("the advocate state", "advocate")).toEqual([
      { text: "the ", match: false },
      { text: "advocate", match: true },
      { text: " state", match: false },
    ]);
  });

  it("is case-insensitive", () => {
    const parts = splitHighlightParts("Advocate STATE", "advocate state");
    expect(parts.filter((p) => p.match).map((p) => p.text)).toEqual([
      "Advocate",
      "STATE",
    ]);
  });
});
