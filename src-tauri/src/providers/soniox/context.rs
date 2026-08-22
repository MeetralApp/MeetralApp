use serde_json::{json, Value};

use crate::providers::shared::live::{
    SonioxContextPayload, SonioxContextProfile, SonioxGeneralPair,
};

/// Soft character budget aligned with Soniox ~8k tokens / ~10k characters docs.
pub const SONIOX_CONTEXT_CHAR_BUDGET: usize = 10_000;

#[derive(Debug, Clone, Default)]
pub struct SonioxContextInput {
    pub general: Vec<SonioxGeneralPair>,
    pub text: String,
    pub terms: Vec<String>,
    pub translation_terms: Vec<(String, String)>,
}

impl From<&SonioxContextPayload> for SonioxContextInput {
    fn from(payload: &SonioxContextPayload) -> Self {
        Self {
            general: payload.general.clone(),
            text: payload.text.clone(),
            terms: payload.terms.clone(),
            translation_terms: payload
                .translation_terms
                .iter()
                .map(|t| (t.source.clone(), t.target.clone()))
                .collect(),
        }
    }
}

/// Approximate serialized size for UI budget (UTF-8 bytes ≈ chars for Latin).
pub fn estimate_context_chars(input: &SonioxContextInput) -> usize {
    let mut n = 0usize;
    for pair in &input.general {
        n = n.saturating_add(pair.key.trim().len());
        n = n.saturating_add(pair.value.trim().len());
    }
    n = n.saturating_add(input.text.trim().len());
    for t in &input.terms {
        n = n.saturating_add(t.trim().len());
    }
    for (s, t) in &input.translation_terms {
        n = n.saturating_add(s.trim().len());
        n = n.saturating_add(t.trim().len());
    }
    n
}

pub fn estimate_payload_chars(payload: &SonioxContextPayload) -> usize {
    estimate_context_chars(&SonioxContextInput::from(payload))
}

/// Merge Always-on with an optional per-meeting profile.
///
/// - `profile == None` → Always-on only
/// - `include_always_on == false` → profile payload only
/// - else: general keys (profile wins), terms/translation union (profile wins),
/// text = always_on then profile (newline-separated)
pub fn merge_soniox_context(
    always_on: &SonioxContextPayload,
    profile: Option<&SonioxContextProfile>,
) -> SonioxContextInput {
    let Some(profile) = profile else {
        return SonioxContextInput::from(always_on);
    };
    if !profile.include_always_on {
        return SonioxContextInput::from(&profile.payload);
    }
    merge_payloads(always_on, &profile.payload)
}

fn merge_payloads(
    always_on: &SonioxContextPayload,
    meeting: &SonioxContextPayload,
) -> SonioxContextInput {
    let mut general: Vec<SonioxGeneralPair> = Vec::new();
    let mut seen_keys: Vec<String> = Vec::new();

    for pair in &meeting.general {
        let key = pair.key.trim();
        if key.is_empty() || pair.value.trim().is_empty() {
            continue;
        }
        let key_owned = key.to_string();
        if seen_keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
            continue;
        }
        seen_keys.push(key_owned.clone());
        general.push(SonioxGeneralPair {
            key: key_owned,
            value: pair.value.trim().to_string(),
        });
    }
    for pair in &always_on.general {
        let key = pair.key.trim();
        if key.is_empty() || pair.value.trim().is_empty() {
            continue;
        }
        if seen_keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
            continue;
        }
        seen_keys.push(key.to_string());
        general.push(SonioxGeneralPair {
            key: key.to_string(),
            value: pair.value.trim().to_string(),
        });
    }

    let ao_text = always_on.text.trim();
    let mt_text = meeting.text.trim();
    let text = match (ao_text.is_empty(), mt_text.is_empty()) {
        (true, true) => String::new(),
        (false, true) => ao_text.to_string(),
        (true, false) => mt_text.to_string(),
        (false, false) => format!("{ao_text}\n{mt_text}"),
    };

    let mut terms: Vec<String> = Vec::new();
    let mut seen_terms: Vec<String> = Vec::new();
    for t in meeting.terms.iter().chain(always_on.terms.iter()) {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if seen_terms.iter().any(|s| s == &lower) {
            continue;
        }
        seen_terms.push(lower);
        terms.push(trimmed.to_string());
    }

    let mut translation_terms: Vec<(String, String)> = Vec::new();
    let mut seen_sources: Vec<String> = Vec::new();
    for term in meeting
        .translation_terms
        .iter()
        .chain(always_on.translation_terms.iter())
    {
        let source = term.source.trim();
        let target = term.target.trim();
        if source.is_empty() || target.is_empty() {
            continue;
        }
        let source_key = source.to_lowercase();
        if seen_sources.iter().any(|s| s == &source_key) {
            continue;
        }
        seen_sources.push(source_key);
        translation_terms.push((source.to_string(), target.to_string()));
    }

    SonioxContextInput {
        general,
        text,
        terms,
        translation_terms,
    }
}

/// Resolve effective context from Always-on + active profile id.
pub fn resolve_active_soniox_context(
    always_on: &SonioxContextPayload,
    profiles: &[SonioxContextProfile],
    active_profile_id: Option<&str>,
) -> SonioxContextInput {
    let profile = active_profile_id.and_then(|id| {
        let id = id.trim();
        if id.is_empty() {
            return None;
        }
        profiles.iter().find(|p| p.id == id)
    });
    merge_soniox_context(always_on, profile)
}

pub fn context_over_budget(input: &SonioxContextInput) -> bool {
    estimate_context_chars(input) > SONIOX_CONTEXT_CHAR_BUDGET
}

/// Build Soniox `context` object from app config fields.
pub fn build_context_object(input: &SonioxContextInput) -> Option<Value> {
    let general: Vec<Value> = input
        .general
        .iter()
        .filter(|p| !p.key.trim().is_empty() && !p.value.trim().is_empty())
        .map(|p| {
            json!({
                "key": p.key.trim(),
                "value": p.value.trim(),
            })
        })
        .collect();

    let text = input.text.trim();

    let terms: Vec<Value> = input
        .terms
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| Value::String(t.to_string()))
        .collect();

    let translation_terms: Vec<Value> = input
        .translation_terms
        .iter()
        .filter(|(s, t)| !s.trim().is_empty() && !t.trim().is_empty())
        .map(|(s, t)| {
            json!({
                "source": s.trim(),
                "target": t.trim(),
            })
        })
        .collect();

    if general.is_empty() && text.is_empty() && terms.is_empty() && translation_terms.is_empty() {
        return None;
    }

    let mut obj = serde_json::Map::new();
    if !general.is_empty() {
        obj.insert("general".into(), Value::Array(general));
    }
    if !text.is_empty() {
        obj.insert("text".into(), Value::String(text.to_string()));
    }
    if !terms.is_empty() {
        obj.insert("terms".into(), Value::Array(terms));
    }
    if !translation_terms.is_empty() {
        obj.insert("translation_terms".into(), Value::Array(translation_terms));
    }
    Some(Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::shared::live::SonioxTranslationTerm;

    fn pair(key: &str, value: &str) -> SonioxGeneralPair {
        SonioxGeneralPair {
            key: key.into(),
            value: value.into(),
        }
    }

    fn term(source: &str, target: &str) -> SonioxTranslationTerm {
        SonioxTranslationTerm {
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn empty_context_is_none() {
        assert!(build_context_object(&SonioxContextInput::default()).is_none());
    }

    #[test]
    fn builds_all_four_sections() {
        let ctx = build_context_object(&SonioxContextInput {
            general: vec![
                pair("domain", "Healthcare"),
                pair("organization", "St John's"),
            ],
            text: "Prior visit notes.".into(),
            terms: vec!["Celebrex".into(), "  ".into()],
            translation_terms: vec![("Mr. Smith".into(), "Sr. Smith".into())],
        })
        .expect("context");
        assert_eq!(ctx["general"].as_array().unwrap().len(), 2);
        assert_eq!(ctx["text"], "Prior visit notes.");
        assert_eq!(ctx["terms"].as_array().unwrap().len(), 1);
        assert_eq!(ctx["translation_terms"][0]["target"], "Sr. Smith");
    }

    #[test]
    fn estimate_counts_trimmed() {
        let n = estimate_context_chars(&SonioxContextInput {
            general: vec![pair("domain", "Health")],
            text: "  ab  ".into(),
            terms: vec!["X".into()],
            translation_terms: vec![("a".into(), "b".into())],
        });
        assert_eq!(n, "domain".len() + "Health".len() + 2 + 1 + 1 + 1);
    }

    #[test]
    fn merge_none_is_always_on() {
        let ao = SonioxContextPayload {
            general: vec![pair("domain", "Healthcare")],
            text: "Always".into(),
            terms: vec!["A".into()],
            translation_terms: vec![term("Mr", "Sr")],
        };
        let merged = merge_soniox_context(&ao, None);
        assert_eq!(merged.general[0].value, "Healthcare");
        assert_eq!(merged.text, "Always");
        assert_eq!(merged.terms, vec!["A".to_string()]);
    }

    #[test]
    fn merge_without_include_is_profile_only() {
        let ao = SonioxContextPayload {
            general: vec![pair("domain", "Healthcare")],
            text: "Always".into(),
            terms: vec!["A".into()],
            translation_terms: vec![],
        };
        let profile = SonioxContextProfile {
            id: "p1".into(),
            name: "Sprint".into(),
            include_always_on: false,
            payload: SonioxContextPayload {
                general: vec![pair("topic", "Sprint")],
                text: "Meeting".into(),
                terms: vec!["B".into()],
                translation_terms: vec![],
            },
        };
        let merged = merge_soniox_context(&ao, Some(&profile));
        assert_eq!(merged.general.len(), 1);
        assert_eq!(merged.general[0].key, "topic");
        assert_eq!(merged.text, "Meeting");
        assert_eq!(merged.terms, vec!["B".to_string()]);
    }

    #[test]
    fn merge_profile_wins_general_and_unions_terms() {
        let ao = SonioxContextPayload {
            general: vec![pair("domain", "Healthcare"), pair("org", "Acme")],
            text: "Always notes".into(),
            terms: vec!["Alpha".into(), "Shared".into()],
            translation_terms: vec![term("Mr. Smith", "Sr. Smith"), term("Hello", "Xin chào")],
        };
        let profile = SonioxContextProfile {
            id: "p1".into(),
            name: "Sprint".into(),
            include_always_on: true,
            payload: SonioxContextPayload {
                general: vec![pair("domain", "Agile")],
                text: "Sprint 42".into(),
                terms: vec!["Shared".into(), "Beta".into()],
                translation_terms: vec![term("Mr. Smith", "Ông Smith")],
            },
        };
        let merged = merge_soniox_context(&ao, Some(&profile));
        assert_eq!(merged.general.len(), 2);
        assert_eq!(merged.general[0].key, "domain");
        assert_eq!(merged.general[0].value, "Agile");
        assert_eq!(merged.general[1].key, "org");
        assert_eq!(merged.text, "Always notes\nSprint 42");
        assert_eq!(
            merged.terms,
            vec![
                "Shared".to_string(),
                "Beta".to_string(),
                "Alpha".to_string()
            ]
        );
        assert_eq!(
            merged.translation_terms,
            vec![
                ("Mr. Smith".into(), "Ông Smith".into()),
                ("Hello".into(), "Xin chào".into()),
            ]
        );
    }

    #[test]
    fn resolve_active_picks_profile() {
        let ao = SonioxContextPayload::default();
        let profiles = vec![SonioxContextProfile {
            id: "p1".into(),
            name: "Sprint".into(),
            include_always_on: false,
            payload: SonioxContextPayload {
                text: "Sprint".into(),
                ..Default::default()
            },
        }];
        let merged = resolve_active_soniox_context(&ao, &profiles, Some("p1"));
        assert_eq!(merged.text, "Sprint");
        let only_ao = resolve_active_soniox_context(&ao, &profiles, None);
        assert!(only_ao.text.is_empty());
    }
}
