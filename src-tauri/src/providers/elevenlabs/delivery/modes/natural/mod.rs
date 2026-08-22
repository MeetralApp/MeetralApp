mod commit;
mod constants;
mod ingest;
mod state;

pub(crate) use commit::commit_ready_sentences;
pub(crate) use ingest::ingest_sentence_mode;
pub(crate) use state::{idle_deadline, NaturalState};
