//! Shared ISO language → ISO 3166-1 alpha-2 country codes for flag icons.
//!
//! One source of truth for Gemini / OpenAI / Soniox language lists and API
//! responses that omit `country_code`.

/// Representative flag country for each ISO language code.
/// Covers Soniox STT/TTS (60+) plus the smaller Gemini/OpenAI sets.
pub const LANGUAGE_FLAG_COUNTRY_CODES: &[(&str, &str)] = &[
    ("af", "ZA"), // Afrikaans — South Africa
    ("sq", "AL"), // Albanian
    ("ar", "SA"), // Arabic
    ("az", "AZ"), // Azerbaijani
    ("eu", "ES"), // Basque
    ("be", "BY"), // Belarusian
    ("bn", "BD"), // Bengali
    ("bs", "BA"), // Bosnian
    ("bg", "BG"), // Bulgarian
    ("ca", "ES"), // Catalan
    ("zh", "CN"), // Chinese
    ("hr", "HR"), // Croatian
    ("cs", "CZ"), // Czech
    ("da", "DK"), // Danish
    ("nl", "NL"), // Dutch
    ("en", "US"), // English
    ("et", "EE"), // Estonian
    ("fi", "FI"), // Finnish
    ("fr", "FR"), // French
    ("gl", "ES"), // Galician
    ("de", "DE"), // German
    ("el", "GR"), // Greek
    ("gu", "IN"), // Gujarati
    ("he", "IL"), // Hebrew
    ("hi", "IN"), // Hindi
    ("hu", "HU"), // Hungarian
    ("id", "ID"), // Indonesian
    ("it", "IT"), // Italian
    ("ja", "JP"), // Japanese
    ("kn", "IN"), // Kannada
    ("kk", "KZ"), // Kazakh
    ("ko", "KR"), // Korean
    ("lv", "LV"), // Latvian
    ("lt", "LT"), // Lithuanian
    ("mk", "MK"), // Macedonian
    ("ms", "MY"), // Malay
    ("ml", "IN"), // Malayalam
    ("mr", "IN"), // Marathi
    ("no", "NO"), // Norwegian
    ("fa", "IR"), // Persian
    ("pl", "PL"), // Polish
    ("pt", "PT"), // Portuguese
    ("pa", "IN"), // Punjabi
    ("ro", "RO"), // Romanian
    ("ru", "RU"), // Russian
    ("sr", "RS"), // Serbian
    ("sk", "SK"), // Slovak
    ("sl", "SI"), // Slovenian
    ("es", "ES"), // Spanish
    ("sw", "KE"), // Swahili
    ("sv", "SE"), // Swedish
    ("tl", "PH"), // Tagalog
    ("ta", "IN"), // Tamil
    ("te", "IN"), // Telugu
    ("th", "TH"), // Thai
    ("tr", "TR"), // Turkish
    ("uk", "UA"), // Ukrainian
    ("ur", "PK"), // Urdu
    ("vi", "VN"), // Vietnamese
    ("cy", "GB"), // Welsh
];

/// Primary language subtag (`en-US` / `en_US` → `en`), lowercased.
pub fn primary_language_code(code: &str) -> String {
    let trimmed = code.trim().to_ascii_lowercase();
    trimmed.split(['-', '_']).next().unwrap_or("").to_string()
}

/// ISO country code for flag display, or empty if unknown.
pub fn country_code_for_language(code: &str) -> String {
    let primary = primary_language_code(code);
    if primary.is_empty() {
        return String::new();
    }
    LANGUAGE_FLAG_COUNTRY_CODES
        .iter()
        .find(|(lang, _)| *lang == primary)
        .map(|(_, country)| (*country).to_string())
        .unwrap_or_default()
}

/// Fill empty `country_code` from the shared map (keeps an explicit value if set).
pub fn resolve_country_code(language_code: &str, existing: &str) -> String {
    let existing = existing.trim();
    if !existing.is_empty() {
        return existing.to_ascii_uppercase();
    }
    country_code_for_language(language_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_core_provider_languages() {
        assert_eq!(country_code_for_language("vi"), "VN");
        assert_eq!(country_code_for_language("en"), "US");
        assert_eq!(country_code_for_language("zh"), "CN");
        assert_eq!(country_code_for_language("ar"), "SA");
    }

    #[test]
    fn handles_regional_variants() {
        assert_eq!(country_code_for_language("en-US"), "US");
        assert_eq!(country_code_for_language("pt_BR"), "PT");
    }

    #[test]
    fn resolve_keeps_explicit_country() {
        assert_eq!(resolve_country_code("en", "GB"), "GB");
        assert_eq!(resolve_country_code("uk", ""), "UA");
    }
}
