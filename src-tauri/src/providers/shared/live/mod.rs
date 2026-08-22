//! Provider-neutral live bridge kernel.
//!
//! Provider slices implement protocol-specific bridges against these shared worker,
//! reconnect, event, and handle primitives. Factory-based connection routing remains
//! in `runtime::factories`; this module has no runtime or command dependency.

pub mod handle;
pub mod reconnect;
pub mod session_error;
pub mod shared;
pub mod worker;

pub use handle::{LiveBridge, LiveBridgeHandle};
pub use reconnect::{
    apply_retry, classify_session_error, meeting_grade_backoff, with_jitter, ReconnectAction,
    ReconnectLoopConfig, ReconnectState,
};
pub use session_error::SessionError;
pub use shared::{
    decode_ws_message, BridgeFatalSender, BridgeStatusEvent, BridgeStatusSender, LiveSetupOptions,
    ReconnectPolicy, TranscriptEvent, TranscriptSender, MAX_RECONNECT_ATTEMPTS,
};
pub use shared::{
    SonioxContextPayload, SonioxContextProfile, SonioxGeneralPair, SonioxLiveSetup,
    SonioxTranslationTerm, StsLiveSetup,
};
pub use worker::{BridgeWorkerCore, AUDIO_IN_CHANNEL_DEPTH};
