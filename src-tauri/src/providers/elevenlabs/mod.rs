pub mod api;
pub mod config;
pub mod delivery;
pub mod protocol;
pub mod worker;

// Back-compat: keep `crate::voice::elevenlabs::latency` resolvable after the
// move of `TurnLatencySlot` into the provider-agnostic `shared::latency`.
pub use crate::voice::shared::latency;

pub use api::{
    list_models, list_voices, preview_voice, validate_voice, ElevenLabsModelOption,
    ElevenLabsVoiceOption,
};
pub use delivery::spawn_outbound_transcript_fanout;
