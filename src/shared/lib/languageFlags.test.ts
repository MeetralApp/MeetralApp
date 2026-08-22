import { describe, expect, it } from "vitest";
import {
  countryCodeForLanguage,
  primaryLanguageCode,
  resolveCountryCode,
  withLanguageFlags,
} from "./languageFlags";

describe("languageFlags", () => {
  it("maps core provider languages to flag countries", () => {
    expect(countryCodeForLanguage("vi")).toBe("VN");
    expect(countryCodeForLanguage("en")).toBe("US");
    expect(countryCodeForLanguage("zh")).toBe("CN");
    expect(countryCodeForLanguage("uk")).toBe("UA");
    expect(countryCodeForLanguage("cy")).toBe("GB");
  });

  it("handles regional variants via primary subtag", () => {
    expect(primaryLanguageCode("en-US")).toBe("en");
    expect(countryCodeForLanguage("pt_BR")).toBe("PT");
  });

  it("resolveCountryCode keeps explicit country codes", () => {
    expect(resolveCountryCode("en", "GB")).toBe("GB");
    expect(resolveCountryCode("vi", "")).toBe("VN");
  });

  it("withLanguageFlags fills missing country codes", () => {
    const enriched = withLanguageFlags([
      { code: "ja", name: "Japanese", countryCode: "" },
      { code: "ko", name: "Korean", countryCode: "KR" },
    ]);
    expect(enriched[0].countryCode).toBe("JP");
    expect(enriched[1].countryCode).toBe("KR");
  });
});
