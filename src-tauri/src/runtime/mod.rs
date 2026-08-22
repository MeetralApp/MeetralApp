//! Runtime composition: session wiring, factories, engine orchestration.
//!
//! `runtime::engine` is the canonical TranslationEngine location.

pub mod control_channel;
pub mod direct_relay;
pub mod engine;
pub mod factories;
pub mod playback_mux;
pub mod voice_runtime;

pub use factories::{
    connect_live_bridge_for, generate_summary, live_setup_for_provider, spawn_outbound_fanout,
    test_live_api_key, FanoutSpawnParams, LiveConnectSetup,
};
pub use voice_runtime::OutboundVoiceRuntime;
