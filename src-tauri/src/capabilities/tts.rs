//! Capability-driven voice/live routing helpers.
//!
//! `PlaybackSource` / fanout kind selection lives here; `match AiProvider` for
//! vendor connect stays in `runtime/factories` and `providers/**`.

use super::catalog::{get_provider_catalog, ProviderCapabilities};
use crate::ai::AiProvider;
use crate::config::{AppConfig, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode};
use crate::voice::config::{VOICE_ENGINE_CUSTOM, VOICE_ENGINE_PROVIDER};

/// Which PCM path feeds Translated playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackSource {
    /// Gemini/OpenAI STS audio from the live bridge.
    BridgeSts,
    /// Separate provider TTS WebSocket (Soniox).
    ProviderTts,
    /// Custom voice TTS (ElevenLabs or Fish Audio).
    CustomTts,
}

impl ProviderCapabilities {
    #[inline]
    pub fn bridge_emits_playback_audio_flag(&self) -> bool {
        self.bridge_emits_playback_audio
    }
}

pub fn live_caps(provider: AiProvider) -> ProviderCapabilities {
    get_provider_catalog(provider).capabilities
}

pub fn uses_separate_tts(provider: AiProvider) -> bool {
    live_caps(provider).uses_separate_tts
}

/// Scaffold separate TTS fanout/worker path for this live provider.
pub fn scaffolds_provider_tts(provider: AiProvider) -> bool {
    uses_separate_tts(provider)
}

pub fn bridge_emits_playback_audio(provider: AiProvider) -> bool {
    live_caps(provider).bridge_emits_playback_audio
}

/// Active Translated playback source for outbound (provider vs custom voice).
pub fn outbound_playback_source(provider: AiProvider, voice_engine: u8) -> PlaybackSource {
    if voice_engine == VOICE_ENGINE_CUSTOM {
        return PlaybackSource::CustomTts;
    }
    if uses_separate_tts(provider) {
        PlaybackSource::ProviderTts
    } else {
        PlaybackSource::BridgeSts
    }
}

/// Active Translated playback source for inbound (provider vs custom voice).
pub fn inbound_playback_source(provider: AiProvider, voice_engine: u8) -> PlaybackSource {
    outbound_playback_source(provider, voice_engine)
}

/// Whether text→TTS commands should be sent (custom voice always; provider TTS when separate).
pub fn tts_text_pipeline_active(
    mode: PipelineOutputMode,
    engine: u8,
    switch_in_progress: bool,
    uses_separate_tts: bool,
) -> bool {
    if switch_in_progress || mode != PipelineOutputMode::Translated {
        return false;
    }
    if engine == VOICE_ENGINE_CUSTOM {
        return true;
    }
    engine == VOICE_ENGINE_PROVIDER && uses_separate_tts
}

pub fn bridge_play_audio_enabled(
    mode: PipelineOutputMode,
    engine: u8,
    bridge_emits_playback_audio: bool,
) -> bool {
    mode == PipelineOutputMode::Translated
        && engine == VOICE_ENGINE_PROVIDER
        && bridge_emits_playback_audio
}

pub fn uses_provider_tts_for_outbound(config: &AppConfig) -> bool {
    uses_separate_tts(config.ai_provider)
        && config.outbound_voice_output == OutboundVoiceOutput::ProviderNative
        && config.outbound_mode == PipelineOutputMode::Translated
}

pub fn uses_provider_tts_for_inbound(config: &AppConfig) -> bool {
    uses_separate_tts(config.ai_provider)
        && config.inbound_voice_output == InboundVoiceOutput::ProviderNative
        && config.inbound_mode == PipelineOutputMode::Translated
}

pub fn needs_custom_tts_for_inbound(config: &AppConfig) -> bool {
    config.needs_custom_tts_for_inbound()
}

pub fn needs_custom_tts_for_outbound(config: &AppConfig) -> bool {
    config.needs_custom_tts_for_outbound()
}

pub fn needs_elevenlabs_for_inbound(config: &AppConfig) -> bool {
    needs_custom_tts_for_inbound(config)
}

pub fn needs_elevenlabs_for_outbound(config: &AppConfig) -> bool {
    needs_custom_tts_for_outbound(config)
}

/// Scaffold inbound text→TTS fanout (Soniox always; Gemini/OpenAI when Custom voice may be used).
///
/// When true, inbound connect tees transcripts and can feed EL or Soniox TTS workers.
pub fn scaffolds_inbound_text_tts(config: &AppConfig) -> bool {
    if scaffolds_provider_tts(config.ai_provider) {
        return true;
    }
    // Gemini/OpenAI: need text fanout for custom voice (and hot-switch Provider↔Custom).
    config.inbound_voice_output.uses_custom_tts()
        || config.is_elevenlabs_api_key_configured()
        || config.is_fishaudio_api_key_configured()
}

/// Which transcript fanout implementation to spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanoutKind {
    /// Soniox AI fanout: prefix-delta stream + one Flush per turn (Custom or Provider TTS).
    ProviderTts,
    /// ElevenLabs Speed/Natural delivery (Gemini/OpenAI live).
    ElevenLabsDelivery,
}

/// Outbound fanout selection.
///
/// Soniox live always uses the Soniox fanout (stream deltas for both Custom and
/// Provider TTS). Gemini/OpenAI use ElevenLabs Speed/Natural delivery.
pub fn outbound_fanout_kind(
    provider: AiProvider,
    _voice_output: OutboundVoiceOutput,
) -> FanoutKind {
    if uses_separate_tts(provider) {
        FanoutKind::ProviderTts
    } else {
        FanoutKind::ElevenLabsDelivery
    }
}

/// Inbound fanout selection when text TTS is scaffolded.
///
/// Soniox → stream fanout. Gemini/OpenAI → EL delivery (cmds gated by voice_engine).
pub fn inbound_fanout_kind(provider: AiProvider, _voice_output: InboundVoiceOutput) -> FanoutKind {
    if uses_separate_tts(provider) {
        FanoutKind::ProviderTts
    } else {
        FanoutKind::ElevenLabsDelivery
    }
}
