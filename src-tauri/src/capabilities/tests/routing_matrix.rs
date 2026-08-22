use crate::ai::AiProvider;
use crate::capabilities::{
    bridge_emits_playback_audio, bridge_play_audio_enabled, outbound_fanout_kind,
    outbound_playback_source, scaffolds_provider_tts, tts_text_pipeline_active,
    uses_provider_tts_for_inbound, uses_provider_tts_for_outbound, uses_separate_tts, FanoutKind,
    PlaybackSource,
};
use crate::config::{AppConfig, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode};
use crate::voice::config::{VOICE_ENGINE_CLONE, VOICE_ENGINE_PROVIDER};

#[test]
fn routing_matrix_playback_source_and_fanout() {
    let cases = [
        (
            AiProvider::Gemini,
            VOICE_ENGINE_PROVIDER,
            OutboundVoiceOutput::ProviderNative,
            PlaybackSource::BridgeSts,
            FanoutKind::ElevenLabsDelivery,
            true,
            false,
        ),
        (
            AiProvider::OpenAi,
            VOICE_ENGINE_PROVIDER,
            OutboundVoiceOutput::ProviderNative,
            PlaybackSource::BridgeSts,
            FanoutKind::ElevenLabsDelivery,
            true,
            false,
        ),
        (
            AiProvider::Soniox,
            VOICE_ENGINE_PROVIDER,
            OutboundVoiceOutput::ProviderNative,
            PlaybackSource::ProviderTts,
            FanoutKind::ProviderTts,
            false,
            true,
        ),
        (
            AiProvider::Gemini,
            VOICE_ENGINE_CLONE,
            OutboundVoiceOutput::ElevenLabsClone,
            PlaybackSource::CloneTts,
            FanoutKind::ElevenLabsDelivery,
            true,
            false,
        ),
        (
            AiProvider::Soniox,
            VOICE_ENGINE_CLONE,
            OutboundVoiceOutput::ElevenLabsClone,
            PlaybackSource::CloneTts,
            FanoutKind::ProviderTts,
            false,
            true,
        ),
    ];

    for (provider, engine, voice_output, source, fanout, bridge_audio, separate_tts) in cases {
        assert_eq!(
            outbound_playback_source(provider, engine),
            source,
            "{provider:?} engine={engine}"
        );
        assert_eq!(
            outbound_fanout_kind(provider, voice_output),
            fanout,
            "{provider:?} voice={voice_output:?}"
        );
        assert_eq!(
            bridge_emits_playback_audio(provider),
            bridge_audio,
            "{provider:?}"
        );
        assert_eq!(uses_separate_tts(provider), separate_tts, "{provider:?}");
        assert_eq!(
            scaffolds_provider_tts(provider),
            separate_tts,
            "{provider:?}"
        );
    }
}

#[test]
fn tts_text_pipeline_active_matrix() {
    assert!(tts_text_pipeline_active(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_CLONE,
        false,
        false, // Gemini: no separate TTS
    ));
    assert!(tts_text_pipeline_active(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_PROVIDER,
        false,
        true, // Soniox: separate TTS
    ));
    assert!(!tts_text_pipeline_active(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_PROVIDER,
        false,
        false, // Gemini: no separate TTS
    ));
    assert!(!tts_text_pipeline_active(
        PipelineOutputMode::TextOnly,
        VOICE_ENGINE_CLONE,
        false,
        false,
    ));
    assert!(!tts_text_pipeline_active(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_CLONE,
        true,
        false,
    ));
}

#[test]
fn bridge_play_audio_enabled_matrix() {
    assert!(bridge_play_audio_enabled(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_PROVIDER,
        true, // Gemini emits bridge playback
    ));
    assert!(bridge_play_audio_enabled(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_PROVIDER,
        true, // OpenAI emits bridge playback
    ));
    assert!(!bridge_play_audio_enabled(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_PROVIDER,
        false, // Soniox: no bridge playback
    ));
    assert!(!bridge_play_audio_enabled(
        PipelineOutputMode::Translated,
        VOICE_ENGINE_CLONE,
        true,
    ));
    assert!(!bridge_play_audio_enabled(
        PipelineOutputMode::OriginalAudio,
        VOICE_ENGINE_PROVIDER,
        true,
    ));
}

#[test]
fn uses_provider_tts_helpers_follow_config() {
    let mut config = AppConfig {
        ai_provider: AiProvider::Soniox,
        outbound_voice_output: OutboundVoiceOutput::ProviderNative,
        outbound_mode: PipelineOutputMode::Translated,
        inbound_mode: PipelineOutputMode::Translated,
        ..AppConfig::default()
    };
    assert!(uses_provider_tts_for_outbound(&config));
    assert!(uses_provider_tts_for_inbound(&config));

    config.outbound_voice_output = OutboundVoiceOutput::ElevenLabsClone;
    assert!(!uses_provider_tts_for_outbound(&config));
    assert!(uses_provider_tts_for_inbound(&config));

    config.inbound_voice_output = InboundVoiceOutput::ElevenLabsClone;
    assert!(!uses_provider_tts_for_inbound(&config));

    config.ai_provider = AiProvider::Gemini;
    config.outbound_voice_output = OutboundVoiceOutput::ProviderNative;
    assert!(!uses_provider_tts_for_outbound(&config));
    assert!(!uses_provider_tts_for_inbound(&config));
}
