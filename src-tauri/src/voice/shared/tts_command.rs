//! Generic text commands fed into a TTS worker's outbound channel.
//!
//! Provider workers (e.g. ElevenLabs `stream-input`) consume these over a
//! bounded `mpsc::Sender<TtsTextCommand>` (see
//! `runtime::control_channel::TTS_CMD_CHANNEL_DEPTH`) and translate them into
//! their vendor-specific WebSocket/HTTP protocol. Keeping the enum here
//! decouples the provider-agnostic TTS delivery layer from any single
//! provider's worker.

/// Feed text to the TTS buffer. Interim deltas use `trigger_generation: false`
/// so providers that batch audio hold off on emitting; hard commits use [`Self::Flush`].
#[derive(Debug)]
pub enum TtsTextCommand {
    AppendDelta {
        text: String,
        trigger_generation: bool,
    },
    Flush,
    Reset,
}
