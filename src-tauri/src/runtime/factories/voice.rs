//! Voice fanout factory — chooses provider-TTS vs ElevenLabs delivery.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8},
    Arc, Mutex,
};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::ai::{AiProvider, TranscriptSender};
use crate::capabilities::{
    inbound_fanout_kind, outbound_fanout_kind, uses_separate_tts, FanoutKind,
};
use crate::config::{InboundVoiceOutput, OutboundVoiceOutput};
use crate::providers::shared::live::TranscriptEvent;
use crate::voice::config::TtsSynthesisMode;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCustomLatencyEvent;
use crate::voice::spawn_outbound_transcript_fanout;

/// Generic alias for the provider-TTS (Soniox) transcript fanout.
pub use crate::voice::spawn_soniox_transcript_fanout as spawn_provider_transcript_fanout;

fn idle_flush_unflushed_text(provider: AiProvider) -> bool {
    matches!(provider, AiProvider::OpenAi)
}

pub struct FanoutSpawnParams {
    pub cancel: CancellationToken,
    pub bridge_rx: mpsc::Receiver<TranscriptEvent>,
    pub engine_tx: TranscriptSender,
    pub tts_cmd_tx: Arc<Mutex<mpsc::Sender<TtsTextCommand>>>,
    pub audio_mode: Arc<AtomicU8>,
    pub voice_engine: Arc<AtomicU8>,
    pub voice_switch_in_progress: Arc<AtomicBool>,
    pub relay_chars_while_provider: Arc<AtomicU64>,
    pub latency_tx: Option<mpsc::Sender<VoiceCustomLatencyEvent>>,
    pub turn_latency: Arc<TurnLatencySlot>,
    pub ai_provider: AiProvider,
    pub voice_output: OutboundVoiceOutput,
    pub synthesis_mode: TtsSynthesisMode,
    pub expected_direction: String,
}

pub struct InboundFanoutSpawnParams {
    pub cancel: CancellationToken,
    pub bridge_rx: mpsc::Receiver<TranscriptEvent>,
    pub engine_tx: TranscriptSender,
    pub tts_cmd_tx: Arc<Mutex<mpsc::Sender<TtsTextCommand>>>,
    pub audio_mode: Arc<AtomicU8>,
    pub voice_engine: Arc<AtomicU8>,
    pub voice_switch_in_progress: Arc<AtomicBool>,
    pub relay_chars_while_provider: Arc<AtomicU64>,
    pub latency_tx: Option<mpsc::Sender<VoiceCustomLatencyEvent>>,
    pub turn_latency: Arc<TurnLatencySlot>,
    pub ai_provider: AiProvider,
    pub voice_output: InboundVoiceOutput,
    pub synthesis_mode: TtsSynthesisMode,
    pub expected_direction: String,
}

/// Spawn the correct transcript→TTS fanout for the live provider / voice output.
pub fn spawn_outbound_fanout(params: FanoutSpawnParams) -> tokio::task::JoinHandle<()> {
    match outbound_fanout_kind(params.ai_provider, params.voice_output) {
        FanoutKind::ProviderTts => spawn_provider_transcript_fanout(
            params.cancel,
            params.bridge_rx,
            params.engine_tx,
            params.tts_cmd_tx,
            params.audio_mode,
            params.voice_engine,
            params.voice_switch_in_progress,
            params.relay_chars_while_provider,
            params.latency_tx,
            params.turn_latency,
            params.expected_direction,
        ),
        FanoutKind::ElevenLabsDelivery => spawn_outbound_transcript_fanout(
            params.cancel,
            params.bridge_rx,
            params.engine_tx,
            params.tts_cmd_tx,
            params.audio_mode,
            params.voice_engine,
            params.voice_switch_in_progress,
            params.relay_chars_while_provider,
            params.latency_tx,
            params.turn_latency,
            uses_separate_tts(params.ai_provider),
            idle_flush_unflushed_text(params.ai_provider),
            params.synthesis_mode,
            params.expected_direction,
        ),
    }
}

/// Spawn the inbound transcript fanout selected by provider capabilities.
pub fn spawn_inbound_fanout(params: InboundFanoutSpawnParams) -> tokio::task::JoinHandle<()> {
    match inbound_fanout_kind(params.ai_provider, params.voice_output) {
        FanoutKind::ProviderTts => spawn_provider_transcript_fanout(
            params.cancel,
            params.bridge_rx,
            params.engine_tx,
            params.tts_cmd_tx,
            params.audio_mode,
            params.voice_engine,
            params.voice_switch_in_progress,
            params.relay_chars_while_provider,
            params.latency_tx,
            params.turn_latency,
            params.expected_direction,
        ),
        FanoutKind::ElevenLabsDelivery => spawn_outbound_transcript_fanout(
            params.cancel,
            params.bridge_rx,
            params.engine_tx,
            params.tts_cmd_tx,
            params.audio_mode,
            params.voice_engine,
            params.voice_switch_in_progress,
            params.relay_chars_while_provider,
            params.latency_tx,
            params.turn_latency,
            uses_separate_tts(params.ai_provider),
            idle_flush_unflushed_text(params.ai_provider),
            params.synthesis_mode,
            params.expected_direction,
        ),
    }
}
