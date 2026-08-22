//! Meeting audio recording — Ogg Opus chunk writer + decode for playback.
//!
//! Format: standard **Ogg Opus** (`.opus`, RFC 7845).

mod encode;
mod resample;
mod session;
mod window;

pub use encode::{
    clear_decode_cache, decode_pcm_f32, decode_pcm_f32_uncached, encode_pcm_i16_to_file,
    is_ogg_file, RECORD_SAMPLE_RATE,
};
pub use session::{
    dropped_recording_chunks, flush_active_recording, maybe_start_for_meeting,
    set_active_recording, start_recording_session, tap_pcm, ActiveRecording, DisabledRecordingTap,
    RecordingTap, SharedActiveRecording,
};
pub use window::{decode_direction_window, decode_window_sync, mix_room, DecodedAudioWindow};

use std::sync::{Arc, Mutex};

pub fn new_shared_active_recording() -> SharedActiveRecording {
    Arc::new(Mutex::new(None))
}
