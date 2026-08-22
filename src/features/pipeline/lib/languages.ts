import {
  countryCodeForLanguage,
  withLanguageFlags,
} from "@/shared/lib/languageFlags";

export interface Language {
  code: string;
  name: string;
  countryCode: string;
}

const FALLBACK_LANGUAGE_DEFS: Array<{ code: string; name: string }> = [
  { code: "en", name: "English" },
  { code: "vi", name: "Vietnamese" },
  { code: "ja", name: "Japanese" },
  { code: "ko", name: "Korean" },
  { code: "zh", name: "Chinese" },
  { code: "fr", name: "French" },
  { code: "de", name: "German" },
  { code: "es", name: "Spanish" },
  { code: "pt", name: "Portuguese" },
  { code: "it", name: "Italian" },
  { code: "ru", name: "Russian" },
  { code: "ar", name: "Arabic" },
  { code: "hi", name: "Hindi" },
  { code: "th", name: "Thai" },
  { code: "id", name: "Indonesian" },
  { code: "nl", name: "Dutch" },
  { code: "pl", name: "Polish" },
  { code: "tr", name: "Turkish" },
];

/** Offline fallback — runtime list comes from provider catalogs (Rust / API). */
export const FALLBACK_LANGUAGES: Language[] = withLanguageFlags(FALLBACK_LANGUAGE_DEFS);

export function getLanguageByCode(
  code: string,
  languages: Language[] = FALLBACK_LANGUAGES,
): Language | undefined {
  const found = languages.find((lang) => lang.code === code);
  if (found) {
    return {
      ...found,
      countryCode:
        found.countryCode || countryCodeForLanguage(found.code),
    };
  }
  const countryCode = countryCodeForLanguage(code);
  if (!countryCode) return undefined;
  return { code, name: code, countryCode };
}

export function swapLanguages(my: string, meeting: string): [string, string] {
  return [meeting, my];
}
