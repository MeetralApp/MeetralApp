//! Translation engine — session coordinator for inbound/outbound pipelines.
//!
//! Implementation is split across seams (keep command APIs stable; avoid large
//! method moves without a dedicated refactor):
//!
//! - [`pipeline_control`] — start/stop inbound & outbound pipelines
//! - [`direct_audio`] — direct-relay audio paths (mic/speaker bypass)
//! - [`publish_state`] — snapshot publish to the frontend
//! - [`watchdog`] — health ticks, device scan, recovery scheduling
//! - [`transcript_relay`] — transcript fan-out and DB writer wiring
//! - [`inbound_voice_switch`] / [`outbound_voice_switch`] — Engine ↔ Clone hot-switch

use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::ai::TranscriptEvent;
use crate::app_state::{AppSnapshot, DeviceCatalogState, PublishContext};
use crate::audio::MicMuteHandle;
use crate::config::AppConfig;
use crate::pipeline::{InboundPipeline, OutboundPipeline};

mod audio_devices;
mod bridge_lifecycle;
mod core;
mod direct_audio;
mod finish_start;
mod helpers;
mod inbound_voice_switch;
mod lock_scope;
mod outbound_voice_switch;
mod pipeline_control;
mod pipeline_start;
mod publish_state;
mod recovery;
mod session_side;
mod shutdown;
mod status;
mod transcript_relay;
mod types;
mod watchdog;

#[cfg(test)]
mod tests;

// --- Voice hot-switch ---
pub use inbound_voice_switch::set_inbound_voice_output;
pub use outbound_voice_switch::set_outbound_voice_output;

// --- Pipeline start helpers ---
pub use finish_start::{run_finish_start_inbound, run_finish_start_outbound};

// --- Watchdog / timing helpers ---
pub use helpers::{
    audio_backoff_ms, config_from_state, format_connection_gap_stamp, unix_ms_now, unix_ms_now_u64,
    watchdog_should_scan_devices, watchdog_tick_sleep_ms, AUDIO_CATALOG_STABLE_MS,
    AUDIO_DIRECT_LOST_MESSAGE, AUDIO_HEARTBEAT_STALE_MS, AUDIO_INBOUND_LOST_MESSAGE,
    AUDIO_OUTBOUND_LOST_MESSAGE, AUDIO_RECOVERY_CONFIRM_MS, AUDIO_STARTUP_GRACE_MS,
    BRIDGE_FATAL_USER_MESSAGE, ENSURE_DIRECT_AUDIO_EVERY_TICKS, MAX_AUDIO_RECONNECT_ATTEMPTS,
    MEETING_IDLE_NO_SEGMENT_MS, PROACTIVE_SESSION_MS, WATCHDOG_DEVICE_SCAN_EVERY_TICKS_IDLE,
    WATCHDOG_DEVICE_SCAN_EVERY_TICKS_STABLE, WATCHDOG_TICK_IDLE_MS, WATCHDOG_TICK_STABLE_MS,
};

pub use lock_scope::{
    apply_audio_path_to_direct_shared, ensure_direct_audio_shared, stop_inbound_shared,
    stop_outbound_shared,
};

// --- Public types ---
pub use audio_devices::AudioDevicesApplyResult;
pub use session_side::{Direction, SessionSide};
pub use types::{
    AppStatus, AudioConnectionState, BridgeConnectionState, DiagnosticsSnapshot, PipelineState,
};

pub struct TranslationEngine {
    outbound: OutboundPipeline,
    inbound: InboundPipeline,
    outbound_side: SessionSide,
    inbound_side: SessionSide,
    status: (PipelineState, PipelineState),
    last_error: Option<String>,
    transcript_tx: Option<mpsc::Sender<TranscriptEvent>>,
    audio_fault_listeners_started: bool,
    watchdog_tick_count: u32,
    mic_mute: MicMuteHandle,
    speaker_mute: MicMuteHandle,
    watchdog_cancel: Option<CancellationToken>,
    pub(crate) cached_device_catalog: Option<DeviceCatalogState>,
    pub(crate) devices_revision: u64,
    pub(crate) config_revision: u64,
    pub(crate) app_state_revision: u64,
    pub(crate) publish_ctx: PublishContext,
    pub(crate) last_published_snapshot: Option<AppSnapshot>,
    pub(crate) ensure_direct_audio_pending: bool,
    /// Last time the enumerated device catalog changed. Ensure/recovery must
    /// wait `AUDIO_CATALOG_STABLE_MS` past this point before acting on it.
    pub(crate) last_catalog_change_ms: Option<u64>,
    transcript_db_writer: Option<crate::meeting::writer_task::TranscriptDbWriterHandle>,
    transcript_db_metrics: crate::meeting::writer_task::SharedTranscriptDbMetrics,
    bridge_reconnect_count: u32,
    /// Shared with pipeline PCM senders — overflow increments (bounded channels).
    pub(crate) pcm_frames_dropped: Arc<std::sync::atomic::AtomicU64>,
    listener_shutdown: CancellationToken,
}

impl Default for TranslationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslationEngine {
    pub fn new() -> Self {
        Self {
            outbound: OutboundPipeline::new(),
            inbound: InboundPipeline::new(),
            outbound_side: SessionSide::new(),
            inbound_side: SessionSide::new(),
            status: (PipelineState::Off, PipelineState::Off),
            last_error: None,
            transcript_tx: None,
            audio_fault_listeners_started: false,
            watchdog_tick_count: 0,
            mic_mute: MicMuteHandle::new(),
            speaker_mute: MicMuteHandle::new(),
            watchdog_cancel: None,
            cached_device_catalog: None,
            devices_revision: 0,
            config_revision: 0,
            app_state_revision: 0,
            publish_ctx: PublishContext::new(AppConfig::default(), Vec::new(), 0),
            last_published_snapshot: None,
            ensure_direct_audio_pending: false,
            last_catalog_change_ms: None,
            transcript_db_writer: None,
            transcript_db_metrics: crate::meeting::writer_task::new_transcript_db_metrics(),
            bridge_reconnect_count: 0,
            pcm_frames_dropped: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            listener_shutdown: CancellationToken::new(),
        }
    }

    pub(super) fn side(&self, direction: Direction) -> &SessionSide {
        match direction {
            Direction::Outbound => &self.outbound_side,
            Direction::Inbound => &self.inbound_side,
        }
    }

    pub(super) fn side_mut(&mut self, direction: Direction) -> &mut SessionSide {
        match direction {
            Direction::Outbound => &mut self.outbound_side,
            Direction::Inbound => &mut self.inbound_side,
        }
    }

    pub fn audio_connection_states(&self) -> (AudioConnectionState, AudioConnectionState) {
        (
            self.outbound_side.audio_path.connection_state(),
            self.inbound_side.audio_path.connection_state(),
        )
    }

    pub fn outbound_voice_runtime(
        &self,
    ) -> Option<Arc<crate::runtime::voice_runtime::OutboundVoiceRuntime>> {
        self.outbound.voice_runtime()
    }

    pub fn pipeline_states(&self) -> (PipelineState, PipelineState) {
        (self.status.0.clone(), self.status.1.clone())
    }

    pub fn runtime_error(&self) -> Option<String> {
        self.last_error.clone()
    }

    pub fn mic_muted(&self) -> bool {
        self.mic_mute.is_muted()
    }

    pub fn speaker_muted(&self) -> bool {
        self.speaker_mute.is_muted()
    }
}

pub type SharedEngine = Arc<Mutex<TranslationEngine>>;

pub fn new_shared_engine() -> SharedEngine {
    Arc::new(Mutex::new(TranslationEngine::new()))
}
