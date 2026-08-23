use serde::{Deserialize, Serialize};

/// How translated text is fed to a TTS provider for outbound custom voice.
///
/// `Streaming` optimises for latency (interim deltas relayed as they arrive);
/// `Sentence` optimises for natural prosody by buffering whole sentences and
/// sending each as one unit + flush, mirroring how a pasted transcript sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TtsSynthesisMode {
    #[default]
    Streaming,
    Sentence,
}

/// Provider native TTS vs custom-voice routing for outbound playback mux.
pub const VOICE_ENGINE_PROVIDER: u8 = 0;
pub const VOICE_ENGINE_CUSTOM: u8 = 1;

pub const EL_PCM_COALESCE_MIN_SAMPLES: usize = 1920; // ~80 ms @ 24 kHz

// Re-export ElevenLabs-specific config so existing `crate::voice::config::*`
// call sites keep resolving after the move into `providers/elevenlabs/config`.
pub use crate::providers::elevenlabs::config::*;
