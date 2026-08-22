//! Per-direction session runtime state.

use tokio_util::sync::CancellationToken;

use crate::app_state::AudioPathPhase;
use crate::audio::AudioFaultSender;
use crate::runtime::direct_relay::DirectRelay;

use super::types::BridgeConnectionState;

/// Pipeline direction for mirrored session control/state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Outbound,
    Inbound,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Outbound => "outbound",
            Self::Inbound => "inbound",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Outbound => Self::Inbound,
            Self::Inbound => Self::Outbound,
        }
    }
}

impl TryFrom<&str> for Direction {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "outbound" => Ok(Self::Outbound),
            "inbound" => Ok(Self::Inbound),
            _ => Err(()),
        }
    }
}

/// Mirrored control/state for one translation direction.
/// Typed pipelines (`OutboundPipeline` / `InboundPipeline`) stay on the engine.
pub struct SessionSide {
    pub(super) direct: DirectRelay,
    pub(super) cancel: Option<CancellationToken>,
    pub(super) starting: bool,
    pub(super) pending_cancel: Option<CancellationToken>,
    pub(super) active_since: Option<i64>,
    pub(super) bridge: BridgeConnectionState,
    pub(super) reconnect_attempt: Option<u32>,
    pub(super) audio_path: AudioPathPhase,
    pub(super) audio_fault_tx: Option<AudioFaultSender>,
}

impl SessionSide {
    pub fn new() -> Self {
        Self {
            direct: DirectRelay::new(),
            cancel: None,
            starting: false,
            pending_cancel: None,
            active_since: None,
            bridge: BridgeConnectionState::Idle,
            reconnect_attempt: None,
            audio_path: AudioPathPhase::Ok,
            audio_fault_tx: None,
        }
    }
}

impl Default for SessionSide {
    fn default() -> Self {
        Self::new()
    }
}
