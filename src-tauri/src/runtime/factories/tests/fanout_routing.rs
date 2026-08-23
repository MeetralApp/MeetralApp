use crate::ai::AiProvider;
use crate::capabilities::{outbound_fanout_kind, FanoutKind};
use crate::config::OutboundVoiceOutput;

#[test]
fn outbound_fanout_kind_maps_providers() {
    // Fanout follows live AI — Soniox always streams prefix deltas (Clone or Provider TTS).
    assert_eq!(
        outbound_fanout_kind(AiProvider::Gemini, OutboundVoiceOutput::ProviderNative),
        FanoutKind::ElevenLabsDelivery
    );
    assert_eq!(
        outbound_fanout_kind(AiProvider::OpenAi, OutboundVoiceOutput::ProviderNative),
        FanoutKind::ElevenLabsDelivery
    );
    assert_eq!(
        outbound_fanout_kind(AiProvider::Soniox, OutboundVoiceOutput::ProviderNative),
        FanoutKind::ProviderTts
    );
    assert_eq!(
        outbound_fanout_kind(AiProvider::Soniox, OutboundVoiceOutput::Custom),
        FanoutKind::ProviderTts
    );
    assert_eq!(
        outbound_fanout_kind(AiProvider::Gemini, OutboundVoiceOutput::Custom),
        FanoutKind::ElevenLabsDelivery
    );
}
