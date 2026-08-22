import { describe, expect, it } from "vitest";
import {
  cleanPayload,
  ensureGeneralRows,
  ensureTranslationRows,
  isSonioxContextOverBudget,
  payloadSummary,
  presetLabel,
} from "./sonioxContext";
import { baseConfig } from "@/test/fixtures/config";
import {
  SONIOX_CONTEXT_CHAR_BUDGET,
  emptySonioxContextPayload,
} from "@/shared/lib/types/pipeline";

describe("cleanPayload", () => {
  it("trims and drops empty rows", () => {
    expect(
      cleanPayload({
        general: [
          { key: "  domain ", value: "  legal " },
          { key: "", value: "x" },
        ],
        text: "  notes  ",
        terms: ["  API ", ""],
        translationTerms: [
          { source: " hi ", target: " chào " },
          { source: "a", target: "" },
        ],
      }),
    ).toEqual({
      general: [{ key: "domain", value: "legal" }],
      text: "notes",
      terms: ["API"],
      translationTerms: [{ source: "hi", target: "chào" }],
    });
  });
});

describe("payloadSummary", () => {
  it("summarizes non-empty sections", () => {
    expect(payloadSummary(emptySonioxContextPayload())).toBe("Empty");
    expect(
      payloadSummary({
        general: [{ key: "domain", value: "legal" }],
        text: "background",
        terms: ["API"],
        translationTerms: [{ source: "a", target: "b" }],
      }),
    ).toBe("1 facts · 1 terms · background · translations");
  });
});

describe("isSonioxContextOverBudget", () => {
  it("detects over-budget merged context", () => {
    const huge = "x".repeat(SONIOX_CONTEXT_CHAR_BUDGET + 10);
    expect(
      isSonioxContextOverBudget({
        ...baseConfig,
        sonioxAlwaysOn: {
          general: [],
          text: huge,
          terms: [],
          translationTerms: [],
        },
      }),
    ).toBe(true);
    expect(
      isSonioxContextOverBudget({
        ...baseConfig,
        sonioxAlwaysOn: emptySonioxContextPayload(),
      }),
    ).toBe(false);
  });
});

describe("ensure rows and presets", () => {
  it("adds empty row when list is empty", () => {
    expect(ensureGeneralRows([], "empty-g")).toEqual([
      { id: "empty-g", key: "", value: "" },
    ]);
    expect(ensureTranslationRows([], "empty-t")).toEqual([
      { id: "empty-t", source: "", target: "" },
    ]);
  });

  it("presetLabel resolves known keys", () => {
    expect(presetLabel("domain")).toBe("Domain");
    expect(presetLabel("unknown")).toBeUndefined();
  });
});
