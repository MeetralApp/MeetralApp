use crate::ai::AiProvider;
use crate::config::{AppConfig, SonioxSettings};
use crate::providers::shared::live::{SonioxContextPayload, SonioxGeneralPair};
use crate::runtime::factories::{live_setup_for_provider, LiveConnectSetup};

fn config_with_soniox_context(provider: AiProvider) -> AppConfig {
    AppConfig {
        ai_provider: provider,
        my_language: "vi".into(),
        meeting_language: "en".into(),
        soniox: SonioxSettings {
            soniox_always_on: SonioxContextPayload {
                general: vec![SonioxGeneralPair {
                    key: "company".into(),
                    value: "Acme".into(),
                }],
                text: "meeting notes".into(),
                terms: vec!["API".into()],
                translation_terms: vec![],
            },
            ..SonioxSettings::default()
        },
        ..AppConfig::default()
    }
}

#[test]
fn live_setup_strips_soniox_fields_for_sts_providers() {
    for provider in [AiProvider::Gemini, AiProvider::OpenAi] {
        let config = config_with_soniox_context(provider);
        let opts = live_setup_for_provider(&config);
        assert!(
            opts.soniox_general.is_empty(),
            "{provider:?} should strip general"
        );
        assert!(
            opts.soniox_context_text.is_empty(),
            "{provider:?} should strip context text"
        );
        assert!(
            opts.soniox_glossary_terms.is_empty(),
            "{provider:?} should strip glossary"
        );
        assert!(
            opts.soniox_translation_terms.is_empty(),
            "{provider:?} should strip translation terms"
        );
        assert!(
            opts.language_hints.is_empty(),
            "{provider:?} should strip language hints"
        );
    }
}

#[test]
fn live_setup_keeps_soniox_fields_for_soniox() {
    let config = config_with_soniox_context(AiProvider::Soniox);
    let opts = live_setup_for_provider(&config);
    assert_eq!(opts.soniox_general.len(), 1);
    assert_eq!(opts.soniox_context_text, "meeting notes");
    assert_eq!(opts.soniox_glossary_terms, vec!["API".to_string()]);
    assert!(!opts.language_hints.is_empty());
}

#[test]
fn live_connect_setup_variants_match_provider() {
    let gemini = LiveConnectSetup::from_config(&AppConfig {
        ai_provider: AiProvider::Gemini,
        ..AppConfig::default()
    });
    assert!(matches!(gemini, LiveConnectSetup::Gemini(_)));

    let openai = LiveConnectSetup::from_config(&AppConfig {
        ai_provider: AiProvider::OpenAi,
        ..AppConfig::default()
    });
    assert!(matches!(openai, LiveConnectSetup::OpenAi(_)));

    let soniox = LiveConnectSetup::from_config(&AppConfig {
        ai_provider: AiProvider::Soniox,
        ..AppConfig::default()
    });
    assert!(matches!(soniox, LiveConnectSetup::Soniox(_)));
}

#[test]
fn live_connect_setup_sts_into_options_strips_soniox() {
    let config = config_with_soniox_context(AiProvider::Gemini);
    let setup = LiveConnectSetup::from_config(&config);
    let LiveConnectSetup::Gemini(sts) = &setup else {
        panic!("expected Gemini StsLiveSetup");
    };
    assert!(!sts.model.is_empty());
    let opts = setup.into_options();
    assert!(opts.soniox_general.is_empty());
    assert!(opts.soniox_context_text.is_empty());
    assert!(opts.language_hints.is_empty());
}

#[test]
fn live_connect_setup_soniox_into_options_keeps_context() {
    let config = config_with_soniox_context(AiProvider::Soniox);
    let setup = LiveConnectSetup::from_config(&config);
    let LiveConnectSetup::Soniox(sx) = &setup else {
        panic!("expected SonioxLiveSetup");
    };
    assert_eq!(sx.soniox_general.len(), 1);
    assert_eq!(sx.soniox_context_text, "meeting notes");
    let opts = setup.into_options();
    assert_eq!(opts.soniox_general.len(), 1);
    assert_eq!(opts.soniox_glossary_terms, vec!["API".to_string()]);
}
