use tokio::sync::mpsc;

use super::events::{BridgeStatusEvent, TranscriptEvent};

// Control-plane channels are bounded: full = drop newest + counted,
// never unbounded memory growth behind a stuck consumer.
pub type TranscriptSender = mpsc::Sender<TranscriptEvent>;
pub type BridgeFatalSender = mpsc::Sender<String>;
pub type BridgeStatusSender = mpsc::Sender<BridgeStatusEvent>;

pub const MAX_RECONNECT_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectPolicy {
    Limited,
    MeetingGrade,
}
