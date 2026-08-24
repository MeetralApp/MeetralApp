use super::{
    AppConfig, ConfigView, DeviceRef, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode,
    TranscriptLayout, VadSensitivity, INPUT_SAMPLE_RATE,
};
use crate::ai::AiProvider;
use crate::ai::DEFAULT_LIVE_MODEL;
use crate::providers::gemini::config::UPLOAD_SAMPLE_RATE;

#[test]
fn needs_playback_for_audio_modes_only() {
    assert!(PipelineOutputMode::Translated.needs_playback());
    assert!(PipelineOutputMode::OriginalAudio.needs_playback());
    assert!(!PipelineOutputMode::TextOnly.needs_playback());
}

#[test]
fn serde_voice_alias_deserializes_to_translated() {
    let mode: PipelineOutputMode = serde_json::from_str("\"voice\"").unwrap();
    assert_eq!(mode, PipelineOutputMode::Translated);
}

#[test]
fn serde_serializes_camel_case_modes() {
    let translated = serde_json::to_string(&PipelineOutputMode::Translated).unwrap();
    let original = serde_json::to_string(&PipelineOutputMode::OriginalAudio).unwrap();
    let text = serde_json::to_string(&PipelineOutputMode::TextOnly).unwrap();
    assert_eq!(translated, "\"translated\"");
    assert_eq!(original, "\"originalAudio\"");
    assert_eq!(text, "\"textOnly\"");
}

#[test]
fn device_ref_empty() {
    assert!(DeviceRef::empty().is_empty());
    assert!(!DeviceRef {
        id: String::new(),
        name: "Mic".into(),
    }
    .is_empty());
}

#[test]
fn gemini_upload_rate_is_one_third_of_local_rate() {
    assert_eq!(INPUT_SAMPLE_RATE / UPLOAD_SAMPLE_RATE, 3);
}

#[test]
fn vad_sensitivity_maps_to_api_values() {
    assert_eq!(VadSensitivity::Low.to_api_value(), "START_SENSITIVITY_LOW");
    assert_eq!(
        VadSensitivity::Low.to_end_api_value(),
        "END_SENSITIVITY_LOW"
    );
    assert_eq!(
        VadSensitivity::Medium.to_api_value(),
        "START_SENSITIVITY_HIGH"
    );
    assert_eq!(
        VadSensitivity::Medium.to_end_api_value(),
        "END_SENSITIVITY_LOW"
    );
    assert_eq!(
        VadSensitivity::High.to_api_value(),
        "START_SENSITIVITY_HIGH"
    );
    assert_eq!(
        VadSensitivity::High.to_end_api_value(),
        "END_SENSITIVITY_HIGH"
    );
}

#[test]
fn normalize_clamps_vad_and_resets_unknown_model() {
    let mut config = AppConfig {
        live_model: "unknown-model".into(),
        summary_model: "unknown-summary".into(),
        vad_silence_duration_ms: 10_000,
        ..AppConfig::default()
    };
    config.normalize();
    assert_eq!(config.live_model, DEFAULT_LIVE_MODEL);
    assert_eq!(config.summary_model, crate::ai::DEFAULT_SUMMARY_MODEL);
    assert_eq!(config.vad_silence_duration_ms, 3000);
}

#[test]
fn normalize_projects_interpreter_and_notes_language_stashes() {
    let mut config = AppConfig {
        session_mode: crate::config::SessionMode::Interpreter,
        my_language: "vi".into(),
        meeting_language: "en".into(),
        interpreter_my_language: "vi".into(),
        interpreter_meeting_language: "en".into(),
        notes_language: "ja".into(),
        outbound_mode: PipelineOutputMode::Translated,
        inbound_mode: PipelineOutputMode::TextOnly,
        interpreter_outbound_mode: Some(PipelineOutputMode::Translated),
        interpreter_inbound_mode: Some(PipelineOutputMode::TextOnly),
        ..AppConfig::default()
    };
    config.normalize();
    assert_eq!(config.my_language, "vi");
    assert_eq!(config.meeting_language, "en");
    assert_eq!(config.outbound_mode, PipelineOutputMode::Translated);
    assert_eq!(config.inbound_mode, PipelineOutputMode::TextOnly);

    config.session_mode = crate::config::SessionMode::Notes;
    config.normalize();
    assert_eq!(config.my_language, "ja");
    assert_eq!(config.meeting_language, "ja");
    assert_eq!(config.outbound_mode, PipelineOutputMode::OriginalAudio);
    assert_eq!(config.inbound_mode, PipelineOutputMode::OriginalAudio);
    // Interpreter stash preserved while Notes is active.
    assert_eq!(config.interpreter_my_language, "vi");
    assert_eq!(config.interpreter_meeting_language, "en");
    assert_eq!(
        config.interpreter_inbound_mode,
        Some(PipelineOutputMode::TextOnly)
    );

    config.session_mode = crate::config::SessionMode::Interpreter;
    config.normalize();
    assert_eq!(config.my_language, "vi");
    assert_eq!(config.meeting_language, "en");
    assert_eq!(config.inbound_mode, PipelineOutputMode::TextOnly);
    assert_eq!(config.notes_language, "ja");
}

#[test]
fn normalize_seeds_mode_stashes_from_legacy_config() {
    let minimal = r#"{
            "geminiApiKey": "",
            "myLanguage": "vi",
            "meetingLanguage": "en",
            "userMic": {"id":"","name":""},
            "teamsMicFeed": {"id":"","name":""},
            "meetingCapture": {"id":"","name":""},
            "localPlayback": {"id":"","name":""}
        }"#;
    let mut config: AppConfig = serde_json::from_str(minimal).unwrap();
    config.normalize();
    assert_eq!(config.interpreter_my_language, "vi");
    assert_eq!(config.interpreter_meeting_language, "en");
    assert_eq!(config.notes_language, "vi");
    assert!(config.interpreter_outbound_mode.is_some());
}

#[test]
fn legacy_flat_soniox_and_elevenlabs_settings_deserialize() {
    let config: AppConfig = serde_json::from_str(
        r#"{
            "geminiApiKey": "",
            "myLanguage": "vi",
            "meetingLanguage": "en",
            "userMic": {"id":"","name":""},
            "teamsMicFeed": {"id":"","name":""},
            "meetingCapture": {"id":"","name":""},
            "localPlayback": {"id":"","name":""},
            "sonioxTtsVoice": "amy",
            "sonioxTtsInboundSpeed": 1.2,
            "sonioxEndpointSensitivity": 0.8,
            "elevenlabsApiKey": "eleven-key",
            "elevenlabsVoiceId": "voice-id",
            "elevenlabsPlaybackCrossfade": true,
            "elevenlabsCrossfadeMs": 12,
            "elevenlabsInboundVoiceId": "inbound-voice"
        }"#,
    )
    .unwrap();

    assert_eq!(config.soniox.soniox_tts_voice, "amy");
    assert!((config.soniox.soniox_tts_inbound_speed - 1.2).abs() < f32::EPSILON);
    assert!((config.soniox.soniox_endpoint_sensitivity - 0.8).abs() < f64::EPSILON);
    assert_eq!(config.elevenlabs.elevenlabs_api_key, "eleven-key");
    assert_eq!(config.elevenlabs.elevenlabs_voice_id, "voice-id");
    assert!(config.elevenlabs.elevenlabs_playback_crossfade);
    assert_eq!(config.elevenlabs.elevenlabs_crossfade_ms, 12);
    assert_eq!(
        config.elevenlabs.elevenlabs_inbound_voice_id,
        "inbound-voice"
    );
}

#[test]
fn setup_options_reflects_config() {
    let config = AppConfig {
        echo_target_language: false,
        vad_silence_duration_ms: 500,
        vad_start_sensitivity: VadSensitivity::Medium,
        vad_end_sensitivity: VadSensitivity::High,
        ..AppConfig::default()
    };
    let options = config.setup_options();
    assert!(!options.echo_target_language);
    assert_eq!(options.vad_silence_duration_ms, 500);
    assert_eq!(options.vad_start_sensitivity, "START_SENSITIVITY_HIGH");
    assert_eq!(options.vad_end_sensitivity, "END_SENSITIVITY_HIGH");
}

#[test]
fn validate_for_start_requires_api_key_and_languages() {
    let mut config = AppConfig::default();
    assert!(config.validate_for_start().is_err());

    config.gemini_api_key = "key".into();
    assert!(config.validate_for_start().is_ok());

    config.my_language.clear();
    assert!(config.validate_for_start().is_err());

    config.my_language = "vi".into();
    config.meeting_language.clear();
    assert!(config.validate_for_start().is_err());

    config.meeting_language = "en".into();
    assert!(config.validate_for_start().is_ok());
}

#[test]
fn transcript_layout_defaults_to_side_by_side() {
    assert_eq!(
        AppConfig::default().transcript_layout,
        TranscriptLayout::SideBySide,
    );

    let minimal = r#"{
            "geminiApiKey": "",
            "myLanguage": "vi",
            "meetingLanguage": "en",
            "userMic": {"id":"","name":""},
            "teamsMicFeed": {"id":"","name":""},
            "meetingCapture": {"id":"","name":""},
            "localPlayback": {"id":"","name":""}
        }"#;
    let config: AppConfig = serde_json::from_str(minimal).unwrap();
    assert_eq!(config.transcript_layout, TranscriptLayout::SideBySide);

    let stacked: AppConfig = serde_json::from_str(
        r#"{"geminiApiKey":"","myLanguage":"vi","meetingLanguage":"en","userMic":{"id":"","name":""},"teamsMicFeed":{"id":"","name":""},"meetingCapture":{"id":"","name":""},"localPlayback":{"id":"","name":""},"transcriptLayout":"stacked"}"#,
    )
    .unwrap();
    assert_eq!(stacked.transcript_layout, TranscriptLayout::Stacked);
}

#[test]
fn uses_provider_tts_helpers() {
    let mut config = AppConfig {
        ai_provider: AiProvider::Soniox,
        outbound_voice_output: OutboundVoiceOutput::ProviderNative,
        outbound_mode: PipelineOutputMode::Translated,
        inbound_mode: PipelineOutputMode::Translated,
        ..AppConfig::default()
    };
    assert!(config.uses_provider_tts_for_outbound());
    assert!(config.uses_provider_tts_for_inbound());

    config.outbound_voice_output = OutboundVoiceOutput::Custom;
    assert!(!config.uses_provider_tts_for_outbound());
    assert!(config.uses_provider_tts_for_inbound());

    config.inbound_voice_output = InboundVoiceOutput::Custom;
    assert!(!config.uses_provider_tts_for_inbound());
    config.inbound_voice_output = InboundVoiceOutput::ProviderNative;

    config.inbound_mode = PipelineOutputMode::TextOnly;
    assert!(!config.uses_provider_tts_for_inbound());

    config.inbound_mode = PipelineOutputMode::OriginalAudio;
    assert!(!config.uses_provider_tts_for_inbound());

    config.ai_provider = AiProvider::Gemini;
    config.outbound_voice_output = OutboundVoiceOutput::ProviderNative;
    config.outbound_mode = PipelineOutputMode::Translated;
    config.inbound_mode = PipelineOutputMode::Translated;
    assert!(!config.uses_provider_tts_for_outbound());
    assert!(!config.uses_provider_tts_for_inbound());
}

#[test]
fn custom_voice_serde_and_vendor_defaults() {
    let outbound: OutboundVoiceOutput = serde_json::from_str("\"custom\"").unwrap();
    assert_eq!(outbound, OutboundVoiceOutput::Custom);
    let inbound: InboundVoiceOutput = serde_json::from_str("\"custom\"").unwrap();
    assert_eq!(inbound, InboundVoiceOutput::Custom);
    assert_eq!(
        serde_json::to_string(&OutboundVoiceOutput::Custom).unwrap(),
        "\"custom\""
    );
    let config = AppConfig::default();
    assert_eq!(
        config.outbound_custom_voice_vendor,
        crate::config::CustomVoiceVendor::ElevenLabs
    );
    assert_eq!(
        config.inbound_custom_voice_vendor,
        crate::config::CustomVoiceVendor::ElevenLabs
    );
}

#[test]
fn validate_custom_voice_mixed_vendors() {
    let mut config = AppConfig {
        outbound_voice_output: OutboundVoiceOutput::Custom,
        inbound_voice_output: InboundVoiceOutput::Custom,
        outbound_mode: PipelineOutputMode::Translated,
        inbound_mode: PipelineOutputMode::Translated,
        outbound_custom_voice_vendor: crate::config::CustomVoiceVendor::FishAudio,
        inbound_custom_voice_vendor: crate::config::CustomVoiceVendor::ElevenLabs,
        ..AppConfig::default()
    };
    let outbound_err = config.validate_custom_voice_outbound_setup().unwrap_err();
    assert!(outbound_err.contains("Fish Audio"));
    let inbound_err = config.validate_custom_voice_inbound_setup().unwrap_err();
    assert!(inbound_err.contains("ElevenLabs"));

    config.elevenlabs.elevenlabs_api_key = "el".into();
    config.elevenlabs.elevenlabs_inbound_voice_id = "el-voice".into();
    config.fishaudio.fishaudio_api_key = "fish".into();
    config.fishaudio.fishaudio_voice_id = "fish-voice".into();
    assert!(config.validate_custom_voice_outbound_setup().is_ok());
    assert!(config.validate_custom_voice_inbound_setup().is_ok());
}

#[test]
fn validate_custom_voice_xai_mixed_with_fish() {
    let mut config = AppConfig {
        outbound_voice_output: OutboundVoiceOutput::Custom,
        inbound_voice_output: InboundVoiceOutput::Custom,
        outbound_mode: PipelineOutputMode::Translated,
        inbound_mode: PipelineOutputMode::Translated,
        outbound_custom_voice_vendor: crate::config::CustomVoiceVendor::Xai,
        inbound_custom_voice_vendor: crate::config::CustomVoiceVendor::FishAudio,
        ..AppConfig::default()
    };
    let outbound_err = config.validate_custom_voice_outbound_setup().unwrap_err();
    assert!(outbound_err.contains("xAI"));
    let inbound_err = config.validate_custom_voice_inbound_setup().unwrap_err();
    assert!(inbound_err.contains("Fish Audio"));

    config.xai.xai_api_key = "xai".into();
    // defaults already set voice_id to "eve"
    config.fishaudio.fishaudio_api_key = "fish".into();
    config.fishaudio.fishaudio_inbound_voice_id = "fish-voice".into();
    assert!(config.validate_custom_voice_outbound_setup().is_ok());
    assert!(config.validate_custom_voice_inbound_setup().is_ok());
}

#[test]
fn config_view_exposes_fallback_key_flags() {
    let config = AppConfig {
        gemini_api_key: "g".into(),
        openai_api_key: String::new(),
        ai_provider: AiProvider::Soniox,
        ..AppConfig::default()
    };
    let view = ConfigView::from(&config);
    assert!(view.gemini_api_key_configured);
    assert!(!view.openai_api_key_configured);
    assert!(view.summary_fallback_available);
}
