use crate::providers::shared::live::{
    SonioxContextPayload, SonioxContextProfile, SonioxGeneralPair, SonioxTranslationTerm,
};

/// Prefer structured `general` pairs; seed from legacy domain/topic when empty.
pub(crate) fn migrate_soniox_general(
    mut general: Vec<SonioxGeneralPair>,
    domain: &str,
    topic: &str,
) -> Vec<SonioxGeneralPair> {
    if !general.is_empty() {
        return general;
    }
    let domain = domain.trim();
    let topic = topic.trim();
    if !domain.is_empty() {
        general.push(SonioxGeneralPair {
            key: "domain".into(),
            value: domain.to_string(),
        });
    }
    if !topic.is_empty() {
        general.push(SonioxGeneralPair {
            key: "topic".into(),
            value: topic.to_string(),
        });
    }
    general
}

fn flat_soniox_non_empty(
    general: &[SonioxGeneralPair],
    text: &str,
    terms: &[String],
    translation: &[SonioxTranslationTerm],
) -> bool {
    general
        .iter()
        .any(|p| !p.key.trim().is_empty() || !p.value.trim().is_empty())
        || !text.trim().is_empty()
        || terms.iter().any(|t| !t.trim().is_empty())
        || translation
            .iter()
            .any(|t| !t.source.trim().is_empty() || !t.target.trim().is_empty())
}

/// Migrate flat legacy context fields into Always-on when the profile shape is still empty.
pub(crate) fn migrate_soniox_always_on(
    always_on: SonioxContextPayload,
    profiles: &[SonioxContextProfile],
    active_id: &Option<String>,
    general: Vec<SonioxGeneralPair>,
    text: String,
    terms: Vec<String>,
    translation_terms: Vec<SonioxTranslationTerm>,
    domain: &str,
    topic: &str,
) -> SonioxContextPayload {
    let already_profile_shape = !always_on.is_empty()
        || !profiles.is_empty()
        || active_id.as_ref().is_some_and(|id| !id.trim().is_empty());
    if already_profile_shape {
        return always_on;
    }
    let general = migrate_soniox_general(general, domain, topic);
    if !flat_soniox_non_empty(&general, &text, &terms, &translation_terms) {
        return always_on;
    }
    SonioxContextPayload {
        general,
        text,
        terms,
        translation_terms,
    }
}
