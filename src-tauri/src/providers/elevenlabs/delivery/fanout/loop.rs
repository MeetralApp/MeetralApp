use std::sync::atomic::Ordering;

use tokio::time::{sleep_until, Instant};
use tracing::info;

use crate::audio::runtime::atomic_to_mode;
use crate::capabilities::tts_text_pipeline_active;
use crate::providers::elevenlabs::delivery::modes::{
    apply_pending_revision, apply_speed_sentence_flush, commit_ready_sentences,
    ensure_segment_text_on_el, idle_deadline, ingest_sentence_mode, ingest_translated_interim,
    segment_fast_lane, FastLaneDebounce, OpenAiIdleFlush,
};
use crate::providers::elevenlabs::delivery::state::RelayState;
use crate::providers::elevenlabs::delivery::tts::{
    emit_turn_latency, flush_pending_before_reset, flush_tts, send_tts_cmd, FlushReason,
};
use crate::voice::config::TtsSynthesisMode;
use crate::voice::shared::tts_command::TtsTextCommand;

use super::context::FanoutContext;

const SHUTDOWN_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

/// Forward remaining bridge events so final turn_complete reaches the transcript relay.
async fn drain_bridge_rx_to_engine(
    bridge_rx: &mut tokio::sync::mpsc::Receiver<crate::providers::shared::live::TranscriptEvent>,
    engine_tx: &crate::ai::TranscriptSender,
) {
    let deadline = Instant::now() + SHUTDOWN_DRAIN_TIMEOUT;
    loop {
        match tokio::time::timeout_at(deadline, bridge_rx.recv()).await {
            Ok(Some(event)) => {
                crate::runtime::control_channel::try_send_control(
                    engine_tx,
                    event,
                    "transcript-fanout",
                );
            }
            Ok(None) => break,
            Err(_) => {
                tracing::debug!("elevenlabs fanout shutdown drain timed out");
                break;
            }
        }
    }
}

pub(crate) async fn run_delivery_loop(ctx: FanoutContext) {
    let FanoutContext {
        cancel,
        mut bridge_rx,
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
        expected_direction,
    } = ctx;

    let mut state = RelayState::new();
    let mut openai_idle_at: Option<Instant> = None;
    let mut revision_debounce_at: Option<Instant> = None;
    let mut fast_lane_at: Option<Instant> = None;
    let mut natural_idle_at: Option<Instant> = None;
    let mut sentence_flush_at: Option<Instant> = None;

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                drain_bridge_rx_to_engine(&mut bridge_rx, &engine_tx).await;
                break;
            }
            _ = async {
                match natural_idle_at {
                    Some(deadline) => sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            }, if natural_idle_at.is_some() => {
                natural_idle_at = None;
                let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
                let engine = voice_engine.load(Ordering::SeqCst);
                let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
                if tts_text_pipeline_active(mode, engine, in_progress, uses_separate_tts)
                    && synthesis_mode == TtsSynthesisMode::Sentence
                    && !state.speed.revision_pending()
                    && state.has_natural_pending()
                {
                    commit_ready_sentences(
                        &tts_cmd_tx,
                        &voice_engine,
                        &relay_chars_while_provider,
                        &mut state,
                        true,
                        "sentence_idle",
                    );
                }
            }
            _ = async {
                match sentence_flush_at {
                    Some(deadline) => sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            }, if sentence_flush_at.is_some() => {
                sentence_flush_at = None;
                let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
                let engine = voice_engine.load(Ordering::SeqCst);
                let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
                if tts_text_pipeline_active(mode, engine, in_progress, uses_separate_tts)
                    && synthesis_mode == TtsSynthesisMode::Streaming
                {
                    apply_speed_sentence_flush(
                        &tts_cmd_tx,
                        &voice_engine,
                        &relay_chars_while_provider,
                        &mut state,
                    );
                }
            }
            _ = async {
                match openai_idle_at {
                    Some(deadline) => sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            }, if openai_idle_at.is_some() => {
                openai_idle_at = None;
                let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
                let engine = voice_engine.load(Ordering::SeqCst);
                let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
                if state.speed.has_unflushed_text
                    && tts_text_pipeline_active(mode, engine, in_progress, uses_separate_tts)
                {
                    if state.speed.revision_pending() {
                        apply_pending_revision(
                            &tts_cmd_tx,
                            &voice_engine,
                            &relay_chars_while_provider,
                            &latency_tx,
                            &turn_latency,
                            &mut state,
                            &mut sentence_flush_at,
                        );
                        revision_debounce_at = None;
                    }
                    flush_tts(
                        &tts_cmd_tx,
                        &voice_engine,
                        &relay_chars_while_provider,
                        &latency_tx,
                        &turn_latency,
                        FlushReason::OpenAiIdleSafety,
                        true,
                        &mut state,
                    );
                }
            }
            _ = async {
                match revision_debounce_at {
                    Some(deadline) => sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            }, if revision_debounce_at.is_some() => {
                revision_debounce_at = None;
                let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
                let engine = voice_engine.load(Ordering::SeqCst);
                let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
                if !tts_text_pipeline_active(mode, engine, in_progress, uses_separate_tts) {
                    continue;
                }
                apply_pending_revision(
                    &tts_cmd_tx,
                    &voice_engine,
                    &relay_chars_while_provider,
                    &latency_tx,
                    &turn_latency,
                    &mut state,
                    &mut sentence_flush_at,
                );
            }
            _ = async {
                match fast_lane_at {
                    Some(deadline) => sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            }, if fast_lane_at.is_some() => {
                fast_lane_at = None;
                let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
                let engine = voice_engine.load(Ordering::SeqCst);
                let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
                if !tts_text_pipeline_active(mode, engine, in_progress, uses_separate_tts) {
                    continue;
                }
                if state.speed.revision_pending() {
                    apply_pending_revision(
                        &tts_cmd_tx,
                        &voice_engine,
                        &relay_chars_while_provider,
                        &latency_tx,
                        &turn_latency,
                        &mut state,
                        &mut sentence_flush_at,
                    );
                    revision_debounce_at = None;
                }
                ensure_segment_text_on_el(
                    &tts_cmd_tx,
                    &voice_engine,
                    &relay_chars_while_provider,
                    &mut state,
                );
                if state.speed.has_unflushed_text {
                    flush_tts(
                        &tts_cmd_tx,
                        &voice_engine,
                        &relay_chars_while_provider,
                        &latency_tx,
                        &turn_latency,
                        FlushReason::SegmentFastLane,
                        true,
                        &mut state,
                    );
                } else {
                    state.reset_phrase(&turn_latency);
                }
                openai_idle_at = None;
            }
            event = bridge_rx.recv() => {
                match event {
                    Some(event) => {
                        if event.direction != expected_direction {
                            crate::runtime::control_channel::try_send_control(
                    &engine_tx,
                    event,
                    "transcript-fanout",
                );
                            continue;
                        }

                        let mode = atomic_to_mode(audio_mode.load(Ordering::SeqCst));
                        let engine = voice_engine.load(Ordering::SeqCst);
                        let in_progress = voice_switch_in_progress.load(Ordering::SeqCst);
                        let tts_active = tts_text_pipeline_active(mode, engine, in_progress, uses_separate_tts);

                        if event.connection_gap {
                            openai_idle_at = None;
                            revision_debounce_at = None;
                            natural_idle_at = None;
                            sentence_flush_at = None;
                            if tts_active {
                                if state.speed.has_unflushed_text {
                                    flush_pending_before_reset(
                                        &tts_cmd_tx,
                                        &voice_engine,
                                        &relay_chars_while_provider,
                                        &mut state,
                                    );
                                }
                                send_tts_cmd(
                                    &tts_cmd_tx,
                                    &voice_engine,
                                    &relay_chars_while_provider,
                                    TtsTextCommand::Reset,
                                );
                            }
                            state.reset_phrase(&turn_latency);
                            state.reset_turn_metrics();
                        }

                        if tts_active {
                            if event.translated_text.is_some() {
                                state.stamp_phrase_start(&turn_latency);
                            }

                            match synthesis_mode {
                                TtsSynthesisMode::Sentence => {
                                    if let Some(ref translated) = event.translated_text {
                                        ingest_sentence_mode(
                                            translated,
                                            &tts_cmd_tx,
                                            &voice_engine,
                                            &relay_chars_while_provider,
                                            &turn_latency,
                                            &mut state,
                                        );
                                        natural_idle_at = idle_deadline(
                                            &state.natural,
                                            &state.tracker,
                                            state.speed.revision_pending(),
                                        );
                                    }

                                    if event.turn_complete {
                                        let had_text = state.natural.committed_bytes > 0
                                            || !state.tracker.last_translated().is_empty();
                                        commit_ready_sentences(
                                            &tts_cmd_tx,
                                            &voice_engine,
                                            &relay_chars_while_provider,
                                            &mut state,
                                            true,
                                            "turn_complete",
                                        );
                                        if had_text {
                                            emit_turn_latency(
                                                &latency_tx,
                                                &turn_latency,
                                                FlushReason::TurnComplete,
                                            );
                                            state.log_turn_metrics();
                                        }
                                        state.reset_turn_metrics();
                                        state.reset_phrase(&turn_latency);
                                        openai_idle_at = None;
                                        revision_debounce_at = None;
                                        fast_lane_at = None;
                                        natural_idle_at = None;
                                        sentence_flush_at = None;
                                    }
                                }
                                TtsSynthesisMode::Streaming => {
                                    if let Some(ref translated) = event.translated_text {
                                        ingest_translated_interim(
                                            translated,
                                            &tts_cmd_tx,
                                            &voice_engine,
                                            &relay_chars_while_provider,
                                            &turn_latency,
                                            &mut state,
                                            &mut revision_debounce_at,
                                            &mut sentence_flush_at,
                                        );
                                    }

                                    if event.turn_complete {
                                        sentence_flush_at = None;
                                        if state.speed.revision_pending() {
                                            apply_pending_revision(
                                                &tts_cmd_tx,
                                                &voice_engine,
                                                &relay_chars_while_provider,
                                                &latency_tx,
                                                &turn_latency,
                                                &mut state,
                                                &mut sentence_flush_at,
                                            );
                                        }
                                        if state.speed.has_unflushed_text {
                                            flush_tts(
                                                &tts_cmd_tx,
                                                &voice_engine,
                                                &relay_chars_while_provider,
                                                &latency_tx,
                                                &turn_latency,
                                                FlushReason::TurnComplete,
                                                true,
                                                &mut state,
                                            );
                                        } else {
                                            state.reset_phrase(&turn_latency);
                                            state.reset_turn_metrics();
                                        }
                                        openai_idle_at = None;
                                        revision_debounce_at = None;
                                        fast_lane_at = None;
                                    } else if segment_fast_lane(&event) {
                                        fast_lane_at =
                                            Some(Instant::now() + FastLaneDebounce);
                                        openai_idle_at = None;
                                    } else {
                                        if event.translated_text.is_some() {
                                            fast_lane_at = None;
                                        }
                                        if idle_flush_unflushed_text
                                            && state.speed.has_unflushed_text
                                        {
                                            openai_idle_at =
                                                Some(Instant::now() + OpenAiIdleFlush);
                                        } else {
                                            openai_idle_at = None;
                                        }
                                    }
                                }
                            }
                        } else if event.turn_complete || segment_fast_lane(&event) {
                            state.reset_phrase(&turn_latency);
                            state.reset_turn_metrics();
                            openai_idle_at = None;
                            revision_debounce_at = None;
                            fast_lane_at = None;
                            natural_idle_at = None;
                            sentence_flush_at = None;
                        }

                        crate::runtime::control_channel::try_send_control(
                            &engine_tx,
                            event,
                            "transcript-fanout",
                        );
                    }
                    None => break,
                }
            }
        }
    }
    info!("tts delivery loop stopped");
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8},
        Arc, Mutex,
    };

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use crate::audio::runtime::mode_to_atomic;
    use crate::capabilities::tts_text_pipeline_active;
    use crate::config::PipelineOutputMode;
    use crate::providers::shared::live::TranscriptEvent;
    use crate::voice::config::{TtsSynthesisMode, VOICE_ENGINE_CUSTOM, VOICE_ENGINE_PROVIDER};
    use crate::voice::shared::latency::TurnLatencySlot;
    use crate::voice::shared::tts_command::TtsTextCommand;
    use crate::voice::spawn_outbound_transcript_fanout;

    #[test]
    fn tts_only_when_translated_mode() {
        assert!(tts_text_pipeline_active(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_CUSTOM,
            false,
            false,
        ));
        assert!(!tts_text_pipeline_active(
            PipelineOutputMode::Translated,
            VOICE_ENGINE_PROVIDER,
            false,
            false,
        ));
        assert_eq!(mode_to_atomic(PipelineOutputMode::Translated), 0);
    }

    fn sample_event(direction: &str, text: &str) -> TranscriptEvent {
        TranscriptEvent {
            direction: direction.to_string(),
            source_text: None,
            translated_text: Some(text.to_string()),
            interim: true,
            turn_complete: false,
            input_segment_finished: false,
            output_segment_finished: false,
            connection_gap: false,
            replace_live: false,
            live_source: None,
            live_translated: None,
        }
    }

    #[tokio::test]
    async fn inbound_expected_direction_drives_tts_outbound_ignored() {
        let cancel = CancellationToken::new();
        let (bridge_tx, bridge_rx) = mpsc::channel(16);
        let (engine_tx, mut engine_rx) = mpsc::channel(16);
        let (tts_tx, mut tts_rx) = mpsc::channel::<TtsTextCommand>(16);
        let tts_cmd_tx = Arc::new(Mutex::new(tts_tx));
        let audio_mode = Arc::new(AtomicU8::new(mode_to_atomic(
            PipelineOutputMode::Translated,
        )));
        let voice_engine = Arc::new(AtomicU8::new(VOICE_ENGINE_PROVIDER));
        let switch_flag = Arc::new(AtomicBool::new(false));
        let relay_chars = Arc::new(AtomicU64::new(0));

        let fanout = spawn_outbound_transcript_fanout(
            cancel.clone(),
            bridge_rx,
            engine_tx,
            tts_cmd_tx,
            audio_mode,
            voice_engine,
            switch_flag,
            relay_chars,
            None,
            TurnLatencySlot::new_shared(),
            true,  // uses_separate_tts — Provider engine drives TTS
            false, // idle_flush_unflushed_text
            TtsSynthesisMode::Streaming,
            "inbound",
        );

        bridge_tx
            .try_send(sample_event("outbound", "skip me"))
            .expect("send outbound");

        let ui_outbound =
            tokio::time::timeout(std::time::Duration::from_millis(500), engine_rx.recv())
                .await
                .expect("ui outbound timeout")
                .expect("ui outbound event");
        assert_eq!(ui_outbound.direction, "outbound");

        bridge_tx
            .try_send(sample_event("inbound", "Hello meeting."))
            .expect("send inbound");

        let ui_inbound =
            tokio::time::timeout(std::time::Duration::from_millis(500), engine_rx.recv())
                .await
                .expect("ui inbound timeout")
                .expect("ui inbound event");
        assert_eq!(ui_inbound.direction, "inbound");

        // The fanout task consumes bridge events sequentially: once the inbound
        // UI event is observed, the outbound iteration has fully completed, so
        // any (buggy) outbound TTS command would already be queued ahead of the
        // inbound one. The unbounded channel's state is final — no sleep needed.
        let cmd = tokio::time::timeout(std::time::Duration::from_millis(500), tts_rx.recv())
            .await
            .expect("tts cmd timeout")
            .expect("tts cmd");
        match cmd {
            TtsTextCommand::AppendDelta { text, .. } => {
                assert!(text.contains("Hello"), "got {text}");
            }
            other => panic!("expected AppendDelta, got {other:?}"),
        }
        assert!(
            tts_rx.try_recv().is_err(),
            "outbound must not drive TTS (only one command, from inbound)"
        );

        cancel.cancel();
        let _ = fanout.await;
    }
}
