pub(crate) mod natural;
pub(crate) mod speed;

pub(crate) use natural::{
    commit_ready_sentences, idle_deadline, ingest_sentence_mode, NaturalState,
};
pub(crate) use speed::{
    apply_pending_revision, apply_speed_sentence_flush, ensure_segment_text_on_el,
    ingest_translated_interim, segment_fast_lane, FastLaneDebounce, OpenAiIdleFlush, SpeedState,
};
