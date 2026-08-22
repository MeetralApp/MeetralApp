mod constants;
mod ingest;
mod revision;
mod state;

pub(crate) use constants::FAST_LANE_DEBOUNCE as FastLaneDebounce;
pub(crate) use constants::OPENAI_IDLE_FLUSH as OpenAiIdleFlush;
pub(crate) use ingest::{
    apply_speed_sentence_flush, ensure_segment_text_on_el, ingest_translated_interim,
    segment_fast_lane,
};
pub(crate) use revision::apply_pending_revision;
pub(crate) use state::SpeedState;
