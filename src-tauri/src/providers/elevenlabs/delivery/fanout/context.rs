use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8},
    Arc, Mutex,
};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::ai::TranscriptSender;
use crate::providers::shared::live::TranscriptEvent;
use crate::voice::config::TtsSynthesisMode;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCustomLatencyEvent;

/// Shared dependencies injected into the delivery event loop.
pub(crate) struct FanoutContext {
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
    pub uses_separate_tts: bool,
    pub idle_flush_unflushed_text: bool,
    pub synthesis_mode: TtsSynthesisMode,
    /// Only events matching this direction drive TTS; others still forward to UI.
    pub expected_direction: String,
}
