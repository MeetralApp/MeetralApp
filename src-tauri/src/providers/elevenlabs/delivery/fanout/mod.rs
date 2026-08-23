mod context;
mod r#loop;

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

use self::context::FanoutContext;
use self::r#loop::run_delivery_loop;

pub fn spawn_outbound_transcript_fanout(
    cancel: CancellationToken,
    bridge_rx: mpsc::Receiver<TranscriptEvent>,
    engine_tx: TranscriptSender,
    tts_cmd_tx: Arc<Mutex<mpsc::Sender<TtsTextCommand>>>,
    audio_mode: Arc<AtomicU8>,
    voice_engine: Arc<AtomicU8>,
    voice_switch_in_progress: Arc<AtomicBool>,
    relay_chars_while_provider: Arc<AtomicU64>,
    latency_tx: Option<mpsc::Sender<VoiceCustomLatencyEvent>>,
    turn_latency: Arc<TurnLatencySlot>,
    uses_separate_tts: bool,
    idle_flush_unflushed_text: bool,
    synthesis_mode: TtsSynthesisMode,
    expected_direction: impl Into<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_delivery_loop(FanoutContext {
        cancel,
        bridge_rx,
        engine_tx,
        tts_cmd_tx,
        audio_mode,
        voice_engine,
        voice_switch_in_progress,
        relay_chars_while_provider,
        latency_tx,
        turn_latency,
        uses_separate_tts,
        idle_flush_unflushed_text,
        synthesis_mode,
        expected_direction: expected_direction.into(),
    }))
}
