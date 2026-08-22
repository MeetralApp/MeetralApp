/**
* Shared ISO language → ISO 3166-1 alpha-2 country codes for flag icons.
* Keep in sync with `src-tauri/src/ai/language_flags.rs`.
*/
export const LANGUAGE_FLAG_COUNTRY_CODES: Record<string, string> = {
  af: "ZA",
  sq: "AL",
  ar: "SA",
  az: "AZ",
  eu: "ES",
  be: "BY",
  bn: "BD",
  bs: "BA",
  bg: "BG",
  ca: "ES",
  zh: "CN",
  hr: "HR",
  cs: "CZ",
  da: "DK",
  nl: "NL",
  en: "US",
  et: "EE",
  fi: "FI",
  fr: "FR",
  gl: "ES",
  de: "DE",
  el: "GR",
  gu: "IN",
  he: "IL",
  hi: "IN",
  hu: "HU",
  id: "ID",
  it: "IT",
  ja: "JP",
  kn: "IN",
  kk: "KZ",
  ko: "KR",
  lv: "LV",
  lt: "LT",
  mk: "MK",
  ms: "MY",
  ml: "IN",
  mr: "IN",
  no: "NO",
  fa: "IR",
  pl: "PL",
  pt: "PT",
  pa: "IN",
  ro: "RO",
  ru: "RU",
  sr: "RS",
  sk: "SK",
  sl: "SI",
  es: "ES",
  sw: "KE",
  sv: "SE",
  tl: "PH",
  ta: "IN",
  te: "IN",
  th: "TH",
  tr: "TR",
  uk: "UA",
  ur: "PK",
  vi: "VN",
  cy: "GB",
};

/** Primary language subtag (`en-US` / `en_US` → `en`). */
export function primaryLanguageCode(code: string): string {
  const trimmed = code.trim().toLowerCase();
  if (!trimmed) return "";
  return trimmed.split(/[-_]/)[0] ?? "";
}

/** ISO country code for flag display, or "" if unknown. */
export function countryCodeForLanguage(code: string): string {
  const primary = primaryLanguageCode(code);
  if (!primary) return "";
  return LANGUAGE_FLAG_COUNTRY_CODES[primary] ?? "";
}

/** Prefer an existing country code; otherwise resolve from the language map. */
export function resolveCountryCode(
  languageCode: string,
  existing?: string | null,
): string {
  const trimmed = existing?.trim();
  if (trimmed) return trimmed.toUpperCase();
  return countryCodeForLanguage(languageCode);
}

export type LanguageFlagFields = {
  code: string;
  name: string;
  countryCode?: string;
};

/** Enrich language entries so UI flag icons always have a country code when known. */
export function withLanguageFlags<T extends LanguageFlagFields>(
  languages: T[],
): Array<T & { countryCode: string }> {
  return languages.map((lang) => ({
    ...lang,
    countryCode: resolveCountryCode(lang.code, lang.countryCode),
  }));
}
