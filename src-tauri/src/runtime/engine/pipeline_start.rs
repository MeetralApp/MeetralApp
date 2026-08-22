use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::types::{BridgeConnectionState, PipelineState};
use super::{unix_ms_now, Direction, SharedEngine, TranslationEngine};
use crate::audio::AudioDeviceInfo;
use crate::config::AppConfig;
use crate::meeting::{ensure_active_meeting, ActiveMeetingId, MeetingStore};
use crate::pipeline::outbound::{await_outbound_session_tasks, OutboundStartConnect};
use crate::providers::shared::live::{BridgeFatalSender, BridgeStatusSender, TranscriptSender};
use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
pub(crate) struct OutboundStartPrepare {
    pub(crate) cancel: CancellationToken,
    pub(crate) transcript_tx: TranscriptSender,
    pub(crate) devices: Vec<AudioDeviceInfo>,
    pub(crate) mic_muted: Arc<AtomicBool>,
    pub(crate) fatal_tx: BridgeFatalSender,
    pub(crate) status_tx: BridgeStatusSender,
}

pub(crate) struct InboundStartPrepare {
    pub(crate) cancel: CancellationToken,
    pub(crate) transcript_tx: TranscriptSender,
    pub(crate) devices: Vec<AudioDeviceInfo>,
    pub(crate) speaker_muted: Arc<AtomicBool>,
    pub(crate) fatal_tx: BridgeFatalSender,
    pub(crate) status_tx: BridgeStatusSender,
}

impl TranslationEngine {
    pub fn begin_start_outbound(
        &mut self,
        config: AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<()> {
        if self.is_direction_busy(Direction::Outbound) {
            return Err(anyhow::anyhow!("Outbound is already running or starting"));
        }

        config
            .validate_for_start()
            .map_err(|e| anyhow::anyhow!(e))?;

        config
            .validate_for_start_outbound(devices)
            .map_err(|e| anyhow::anyhow!(e))?;

        let side = self.side_mut(Direction::Outbound);
        side.starting = true;
        side.pending_cancel = Some(CancellationToken::new());
        self.status.0 = PipelineState::Starting;
        self.last_error = None;
        self.ensure_transcript_relay(app);
        if let Some(engine) = app.try_state::<std::sync::Arc<crate::meeting::SharedSegmentEngine>>()
        {
            engine.open_direction("outbound");
        }
        if let (Some(store), Some(active)) = (
            app.try_state::<std::sync::Arc<MeetingStore>>(),
            app.try_state::<ActiveMeetingId>(),
        ) {
            if let Err(e) = ensure_active_meeting(app, store.inner(), active.inner(), &config) {
                tracing::error!("failed to ensure active meeting on outbound start: {e:#}");
                let _ = app.emit("meeting-error", e.to_string());
            }
        }
        self.publish_state(app);
        Ok(())
    }

    pub fn begin_start_inbound(
        &mut self,
        config: AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) -> Result<()> {
        if self.is_direction_busy(Direction::Inbound) {
            return Err(anyhow::anyhow!("Inbound is already running or starting"));
        }

        config
            .validate_for_start()
            .map_err(|e| anyhow::anyhow!(e))?;

        config
            .validate_for_start_inbound(devices)
            .map_err(|e| anyhow::anyhow!(e))?;

        let side = self.side_mut(Direction::Inbound);
        side.starting = true;
        side.pending_cancel = Some(CancellationToken::new());
        self.status.1 = PipelineState::Starting;
        self.last_error = None;
        self.ensure_transcript_relay(app);
        if let Some(engine) = app.try_state::<std::sync::Arc<crate::meeting::SharedSegmentEngine>>()
        {
            engine.open_direction("inbound");
        }
        if let (Some(store), Some(active)) = (
            app.try_state::<std::sync::Arc<MeetingStore>>(),
            app.try_state::<ActiveMeetingId>(),
        ) {
            if let Err(e) = ensure_active_meeting(app, store.inner(), active.inner(), &config) {
                tracing::error!("failed to ensure active meeting on inbound start: {e:#}");
                let _ = app.emit("meeting-error", e.to_string());
            }
        }
        self.publish_state(app);
        Ok(())
    }

    pub(super) async fn fail_start(
        &mut self,
        direction: Direction,
        error: String,
        config: &AppConfig,
        devices: &[AudioDeviceInfo],
        app: &AppHandle,
    ) {
        let side = self.side_mut(direction);
        side.starting = false;
        side.active_since = None;
        match direction {
            Direction::Outbound => self.status.0 = PipelineState::Error,
            Direction::Inbound => self.status.1 = PipelineState::Error,
        }
        self.clear_bridge_state(direction);
        self.last_error = Some(error);
        self.clear_transcript_relay_if_idle();
        match direction {
            Direction::Outbound => {
                self.resume_direct_outbound_if_enabled(config, devices, app);
            }
            Direction::Inbound => {
                self.resume_direct_inbound_if_enabled(config, devices, app);
            }
        }
        self.publish_state(app);
    }

    pub(super) async fn abort_start_after_connect<F>(
        &mut self,
        direction: Direction,
        cancel: CancellationToken,
        cleanup: F,
        app: &AppHandle,
    ) where
        F: Future<Output = ()>,
    {
        cancel.cancel();
        cleanup.await;
        self.side_mut(direction).starting = false;
        match direction {
            Direction::Outbound if self.status.0 == PipelineState::Starting => {
                self.status.0 = PipelineState::Off;
            }
            Direction::Inbound if self.status.1 == PipelineState::Starting => {
                self.status.1 = PipelineState::Off;
            }
            _ => {}
        }
        self.clear_transcript_relay_if_idle();
        self.publish_state(app);
    }

    pub(super) fn prepare_finish_start_outbound(
        &mut self,
        app: &AppHandle,
        engine: SharedEngine,
        devices: &[AudioDeviceInfo],
    ) -> Result<Option<OutboundStartPrepare>, String> {
        let Some(cancel) = self.outbound_side.pending_cancel.take() else {
            self.outbound_side.starting = false;
            self.clear_transcript_relay_if_idle();
            self.publish_state(app);
            return Ok(None);
        };

        if !self.outbound_side.starting {
            cancel.cancel();
            self.clear_transcript_relay_if_idle();
            self.publish_state(app);
            return Ok(None);
        }

        let Some(transcript_tx) = self.transcript_tx.clone() else {
            tracing::error!("outbound start aborted: transcript relay missing");
            cancel.cancel();
            self.outbound_side.starting = false;
            self.clear_transcript_relay_if_idle();
            self.publish_state(app);
            return Ok(None);
        };

        let devices = devices.to_vec();

        let (fatal_tx, fatal_rx) =
            mpsc::channel(crate::runtime::control_channel::BRIDGE_FATAL_CHANNEL_DEPTH);
        Self::spawn_fatal_listener(Direction::Outbound, engine.clone(), app.clone(), fatal_rx);

        let (status_tx, status_rx) =
            mpsc::channel(crate::runtime::control_channel::BRIDGE_STATUS_CHANNEL_DEPTH);
        Self::spawn_bridge_status_listener(engine, app.clone(), status_rx);

        Ok(Some(OutboundStartPrepare {
            cancel,
            transcript_tx,
            devices,
            mic_muted: self.mic_mute.shared(),
            fatal_tx,
            status_tx,
        }))
    }

    pub(super) async fn complete_finish_start_outbound(
        &mut self,
        prepare: OutboundStartPrepare,
        config: AppConfig,
        app: &AppHandle,
        bridge_result: Result<OutboundStartConnect, anyhow::Error>,
    ) {
        if !self.outbound_side.starting || prepare.cancel.is_cancelled() {
            if let Ok(mut connected) = bridge_result {
                self.abort_start_after_connect(
                    Direction::Outbound,
                    prepare.cancel,
                    async move {
                        connected.bridge.stop().await;
                        drop(connected.playback_rx);
                        if let Some(tasks) = connected.session_tasks.take() {
                            await_outbound_session_tasks(tasks).await;
                        }
                    },
                    app,
                )
                .await;
            } else {
                prepare.cancel.cancel();
                self.outbound_side.starting = false;
                self.clear_transcript_relay_if_idle();
                self.publish_state(app);
            }
            return;
        }

        match bridge_result {
            Ok(mut connected) => {
                if !self.outbound_side.starting || prepare.cancel.is_cancelled() {
                    self.abort_start_after_connect(
                        Direction::Outbound,
                        prepare.cancel,
                        async move {
                            connected.bridge.stop().await;
                            drop(connected.playback_rx);
                            if let Some(tasks) = connected.session_tasks.take() {
                                await_outbound_session_tasks(tasks).await;
                            }
                        },
                        app,
                    )
                    .await;
                    return;
                }

                if let Some(rx) = connected.voice_tts_status_rx.take() {
                    Self::spawn_voice_tts_status_listener(app.clone(), rx);
                }
                if let Some(rx) = connected.voice_latency_rx.take() {
                    Self::spawn_voice_clone_latency_listener(app.clone(), rx);
                }

                self.stop_direct_outbound_internal().await;
                match self
                    .outbound
                    .start_with_bridge(
                        &config,
                        &prepare.devices,
                        connected,
                        prepare.cancel.clone(),
                        prepare.mic_muted,
                        self.audio_fault_tx(Direction::Outbound),
                    )
                    .await
                {
                    Ok(()) => {
                        self.status.0 = PipelineState::Active;
                        let side = self.side_mut(Direction::Outbound);
                        side.active_since = Some(unix_ms_now());
                        side.bridge = BridgeConnectionState::Ready;
                        side.reconnect_attempt = None;
                        side.cancel = Some(prepare.cancel);
                        self.clear_audio_state(Direction::Outbound);
                    }
                    Err(e) => {
                        prepare.cancel.cancel();
                        self.outbound.stop().await;
                        self.fail_start(
                            Direction::Outbound,
                            e.to_string(),
                            &config,
                            &prepare.devices,
                            app,
                        )
                        .await;
                        return;
                    }
                }
            }
            Err(e) => {
                prepare.cancel.cancel();
                self.fail_start(
                    Direction::Outbound,
                    e.to_string(),
                    &config,
                    &prepare.devices,
                    app,
                )
                .await;
                return;
            }
        }

        self.outbound_side.starting = false;
        self.clear_transcript_relay_if_idle();
        self.publish_state(app);
    }

    pub(super) fn prepare_finish_start_inbound(
        &mut self,
        app: &AppHandle,
        engine: SharedEngine,
        devices: &[AudioDeviceInfo],
    ) -> Result<Option<InboundStartPrepare>, String> {
        let Some(cancel) = self.inbound_side.pending_cancel.take() else {
            self.inbound_side.starting = false;
            self.clear_transcript_relay_if_idle();
            self.publish_state(app);
            return Ok(None);
        };

        if !self.inbound_side.starting {
            cancel.cancel();
            self.clear_transcript_relay_if_idle();
            self.publish_state(app);
            return Ok(None);
        }

        let Some(transcript_tx) = self.transcript_tx.clone() else {
            tracing::error!("inbound start aborted: transcript relay missing");
            cancel.cancel();
            self.inbound_side.starting = false;
            self.clear_transcript_relay_if_idle();
            self.publish_state(app);
            return Ok(None);
        };

        let devices = devices.to_vec();

        let (fatal_tx, fatal_rx) =
            mpsc::channel(crate::runtime::control_channel::BRIDGE_FATAL_CHANNEL_DEPTH);
        Self::spawn_fatal_listener(Direction::Inbound, engine.clone(), app.clone(), fatal_rx);

        let (status_tx, status_rx) =
            mpsc::channel(crate::runtime::control_channel::BRIDGE_STATUS_CHANNEL_DEPTH);
        Self::spawn_bridge_status_listener(engine, app.clone(), status_rx);

        Ok(Some(InboundStartPrepare {
            cancel,
            transcript_tx,
            devices,
            speaker_muted: self.speaker_mute.shared(),
            fatal_tx,
            status_tx,
        }))
    }

    pub(super) async fn complete_finish_start_inbound(
        &mut self,
        prepare: InboundStartPrepare,
        config: AppConfig,
        app: &AppHandle,
        bridge_result: Result<crate::pipeline::inbound::InboundBridgeConnect, anyhow::Error>,
    ) {
        if !self.inbound_side.starting || prepare.cancel.is_cancelled() {
            if let Ok(connected) = bridge_result {
                self.abort_start_after_connect(
                    Direction::Inbound,
                    prepare.cancel,
                    async move {
                        connected.bridge.stop().await;
                        drop(connected.playback_rx);
                        drop(connected.playback_chunks);
                        drop(connected.audio_mode);
                    },
                    app,
                )
                .await;
            } else {
                prepare.cancel.cancel();
                self.inbound_side.starting = false;
                self.clear_transcript_relay_if_idle();
                self.publish_state(app);
            }
            return;
        }

        match bridge_result {
            Ok(connected) => {
                if !self.inbound_side.starting || prepare.cancel.is_cancelled() {
                    self.abort_start_after_connect(
                        Direction::Inbound,
                        prepare.cancel,
                        async move {
                            connected.bridge.stop().await;
                            drop(connected.playback_rx);
                            drop(connected.playback_chunks);
                            drop(connected.audio_mode);
                        },
                        app,
                    )
                    .await;
                    return;
                }

                self.stop_direct_inbound_internal().await;
                match self
                    .inbound
                    .start_with_bridge(
                        &config,
                        &prepare.devices,
                        connected,
                        prepare.cancel.clone(),
                        prepare.speaker_muted,
                        self.audio_fault_tx(Direction::Inbound),
                        app.clone(),
                    )
                    .await
                {
                    Ok(()) => {
                        self.status.1 = PipelineState::Active;
                        let side = self.side_mut(Direction::Inbound);
                        side.active_since = Some(unix_ms_now());
                        side.bridge = BridgeConnectionState::Ready;
                        side.reconnect_attempt = None;
                        side.cancel = Some(prepare.cancel);
                        self.clear_audio_state(Direction::Inbound);
                    }
                    Err(e) => {
                        prepare.cancel.cancel();
                        self.inbound.stop().await;
                        self.fail_start(
                            Direction::Inbound,
                            e.to_string(),
                            &config,
                            &prepare.devices,
                            app,
                        )
                        .await;
                        return;
                    }
                }
            }
            Err(e) => {
                prepare.cancel.cancel();
                self.fail_start(
                    Direction::Inbound,
                    e.to_string(),
                    &config,
                    &prepare.devices,
                    app,
                )
                .await;
                return;
            }
        }

        self.inbound_side.starting = false;
        self.clear_transcript_relay_if_idle();
        self.publish_state(app);
    }
}
