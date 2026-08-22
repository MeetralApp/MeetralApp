import { describe, expect, it } from "vitest";
import {
  FALLBACK_LANGUAGES,
  getLanguageByCode,
  swapLanguages,
} from "./languages";
import { countryCodeForLanguage } from "@/shared/lib/languageFlags";

describe("getLanguageByCode", () => {
  it("returns a known language", () => {
    expect(getLanguageByCode("vi")).toEqual({
      code: "vi",
      name: "Vietnamese",
      countryCode: "VN",
    });
  });

  it("returns undefined for unknown codes", () => {
    expect(getLanguageByCode("xx")).toBeUndefined();
  });

  it("covers every supported language entry", () => {
    for (const lang of FALLBACK_LANGUAGES) {
      expect(getLanguageByCode(lang.code)).toEqual(lang);
      expect(lang.countryCode).toBe(countryCodeForLanguage(lang.code));
    }
  });
});

describe("swapLanguages", () => {
  it("exchanges my and meeting language codes", () => {
    expect(swapLanguages("vi", "en")).toEqual(["en", "vi"]);
    expect(swapLanguages("ja", "ja")).toEqual(["ja", "ja"]);
  });
});
