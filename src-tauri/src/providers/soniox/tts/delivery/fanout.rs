//! Turn-scoped Soniox transcript → TTS command fanout.
//!
//! Peels interim translated text into prefix `AppendDelta`s and one `Flush` per
//! utterance end. Same path for ElevenLabs Clone and Soniox Provider TTS —
//! sentence buffering is intentionally avoided for low TTFB.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex,
};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::ai::TranscriptSender;
use crate::audio::runtime::atomic_to_mode;
use crate::capabilities::tts_text_pipeline_active;
use crate::providers::shared::live::TranscriptEvent;
use crate::voice::shared::debug;
use crate::voice::shared::latency::TurnLatencySlot;
use crate::voice::shared::tts_command::TtsTextCommand;
use crate::voice::shared::types::VoiceCustomLatencyEvent;

use super::stream::{flush_stream_turn, ingest_stream_delta, StreamState};

const SHUTDOWN_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

fn send_cmd(tts_cmd_tx: &Mutex<mpsc::Sender<TtsTextCommand>>, cmd: TtsTextCommand) {
    if let Ok(tx) = tts_cmd_tx.lock() {
        crate::runtime::control_channel::try_send_control(&tx, cmd, "tts-cmd");
    }
}

fn handle_event(
    event: TranscriptEvent,
    expected_direction: &str,
    engine_tx: &TranscriptSender,
    tts_cmd_tx: &Mutex<mpsc::Sender<TtsTextCommand>>,
    audio_mode: &AtomicU8,
    voice_engine: &AtomicU8,
    voice_switch_in_progress: &AtomicBool,
    latency_tx: &Option<mpsc::Sender<VoiceCustomLatencyEvent>>,
    turn_latency: &TurnLatencySlot,
    state: &mut StreamState,
) {
    if event.direction != expected_direction {
        crate::runtime::control_channel::try_send_control(engine_tx, event, "transcript-fanout");
        return;
    }

    let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
    let engine = voice_engine.load(Ordering::SeqCst);
    let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
    let tts_active = tts_text_pipeline_active(
        mode,
        engine,
        in_progress,
        true, // this fanout is only for separate-TTS providers
    );

    if event.connection_gap {
        if tts_active && state.has_unflushed() {
            send_cmd(tts_cmd_tx, TtsTextCommand::Flush);
        }
        if tts_active {
            debug::log_soniox_delivery("connection_gap_reset");
            send_cmd(tts_cmd_tx, TtsTextCommand::Reset);
        }
        state.reset();
        turn_latency.reset();
    }

    if tts_active {
        if let Some(ref translated) = event.translated_text {
            ingest_stream_delta(translated, tts_cmd_tx, turn_latency, state);
        }
        if event.turn_complete {
            flush_stream_turn(tts_cmd_tx, state, latency_tx, turn_latency);
        }
    } else if event.turn_complete || event.connection_gap {
        state.reset();
    }

    crate::runtime::control_channel::try_send_control(engine_tx, event, "transcript-fanout");
}

/// Drain remaining bridge events after cancel so final turn_complete reaches the relay.
async fn drain_bridge_rx(
    bridge_rx: &mut mpsc::Receiver<TranscriptEvent>,
    expected_direction: &str,
    engine_tx: &TranscriptSender,
    tts_cmd_tx: &Mutex<mpsc::Sender<TtsTextCommand>>,
    audio_mode: &AtomicU8,
    voice_engine: &AtomicU8,
    voice_switch_in_progress: &AtomicBool,
    latency_tx: &Option<mpsc::Sender<VoiceCustomLatencyEvent>>,
    turn_latency: &TurnLatencySlot,
    state: &mut StreamState,
) {
    let deadline = tokio::time::Instant::now() + SHUTDOWN_DRAIN_TIMEOUT;
    loop {
        match tokio::time::timeout_at(deadline, bridge_rx.recv()).await {
            Ok(Some(event)) => {
                handle_event(
                    event,
                    expected_direction,
                    engine_tx,
                    tts_cmd_tx,
                    audio_mode,
                    voice_engine,
                    voice_switch_in_progress,
                    latency_tx,
                    turn_latency,
                    state,
                );
            }
            Ok(None) => break,
            Err(_) => {
                tracing::debug!("soniox fanout shutdown drain timed out");
                break;
            }
        }
    }
}

/// Provider-TTS transcript fanout — prefix deltas + one Flush per turn.
pub fn spawn_soniox_transcript_fanout(
    cancel: CancellationToken,
    bridge_rx: mpsc::Receiver<TranscriptEvent>,
    engine_tx: TranscriptSender,
    tts_cmd_tx: Arc<Mutex<mpsc::Sender<TtsTextCommand>>>,
    audio_mode: Arc<AtomicU8>,
    voice_engine: Arc<AtomicU8>,
    voice_switch_in_progress: Arc<AtomicBool>,
    _relay_chars_while_provider: Arc<AtomicU64>,
    latency_tx: Option<mpsc::Sender<VoiceCustomLatencyEvent>>,
    turn_latency: Arc<TurnLatencySlot>,
    expected_direction: impl Into<String>,
) -> tokio::task::JoinHandle<()> {
    let expected_direction = expected_direction.into();
    tokio::spawn(async move {
        let mut bridge_rx = bridge_rx;
        let mut state = StreamState::new();

        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    drain_bridge_rx(
                        &mut bridge_rx,
                        &expected_direction,
                        &engine_tx,
                        &tts_cmd_tx,
                        &audio_mode,
                        &voice_engine,
                        &voice_switch_in_progress,
                        &latency_tx,
                        &turn_latency,
                        &mut state,
                    ).await;
                    break;
                }
                event = bridge_rx.recv() => {
                    match event {
                        Some(event) => {
                            handle_event(
                                event,
                                &expected_direction,
                                &engine_tx,
                                &tts_cmd_tx,
                                &audio_mode,
                                &voice_engine,
                                &voice_switch_in_progress,
                                &latency_tx,
                                &turn_latency,
                                &mut state,
                            );
                        }
                        None => break,
                    }
                }
            }
        }
        info!("soniox tts delivery loop stopped");
    })
}
