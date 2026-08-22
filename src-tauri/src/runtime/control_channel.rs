//! Bounded control-channel policy.
//!
//! Control/status/TTS-command channels are bounded so a stuck consumer cannot
//! grow memory without bound. PCM data-plane channels keep their existing
//! bounded + drop-on-full realtime policy; here we cover the control plane.
//! A full channel drops the *newest* message, counts it, and logs at a
//! rate-limited cadence — stale-control coalescing is done at the consumers.

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

/// TTS text/command backlog per worker (tens of seconds of speech).
pub const TTS_CMD_CHANNEL_DEPTH: usize = 64;
/// Worker status fanout; consumers drain immediately.
pub const TTS_STATUS_CHANNEL_DEPTH: usize = 32;
/// Bridge fatal one-shots — the first fatal is always delivered; later ones
/// are redundant (the fatal listener consumes one message then exits).
pub const BRIDGE_FATAL_CHANNEL_DEPTH: usize = 1;
/// Bridge status transitions (reconnect attempts arrive at backoff cadence).
pub const BRIDGE_STATUS_CHANNEL_DEPTH: usize = 8;
/// Audio fault bursts from capture threads; the first fault drives recovery,
/// later faults during recovery are no-ops.
pub const AUDIO_FAULT_CHANNEL_DEPTH: usize = 32;
/// Live transcript fanout to the UI; the FE coalesces at 80ms cadence.
pub const TRANSCRIPT_FANOUT_CHANNEL_DEPTH: usize = 256;
/// Voice clone latency events (low rate).
pub const VOICE_LATENCY_CHANNEL_DEPTH: usize = 32;

static DROPS: AtomicU64 = AtomicU64::new(0);

/// Total control-channel drops since process start (diagnostics surface).
pub fn control_channel_drops() -> u64 {
    DROPS.load(Ordering::Relaxed)
}

/// `try_send` with drop accounting: on a full channel the newest message is
/// dropped, the global counter increments, and a rate-limited warning logs
/// the channel label for per-channel attribution.
pub fn try_send_control<T>(tx: &mpsc::Sender<T>, msg: T, label: &'static str) {
    match tx.try_send(msg) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            let drops = DROPS.fetch_add(1, Ordering::Relaxed) + 1;
            if drops == 1 || drops.is_multiple_of(50) {
                tracing::warn!(
                    channel = label,
                    drops,
                    "control channel full; dropping message"
                );
            }
        }
        // Receiver already gone — expected during worker teardown/restart
        // (senders race the session stop), not backpressure. Keep out of the
        // drop counter so "full" stays a genuine backpressure signal.
        Err(mpsc::error::TrySendError::Closed(_)) => {
            tracing::debug!(channel = label, "control channel closed; skipping message");
        }
    }
}
