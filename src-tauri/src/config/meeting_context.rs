//! App-level meeting context (domain / terminology) for LLM surfaces.
//!
//! Shares the four-section context model (`general` key-value pairs / free-form
//! `text` / `terms` / `translation_terms`) with the Soniox STT session payload —
//! the same industry model Soniox documents for domain adaptation. The canonical
//! structs live in `providers::shared::live` (config already depends on that
//! module); this module re-exports them under neutral names so non-Soniox
//! consumers (summary prompts, settings view-model) don't carry STT naming.

pub use crate::providers::shared::live::{
    SonioxContextPayload as MeetingContextPayload, SonioxGeneralPair as MeetingContextPair,
    SonioxTranslationTerm as MeetingContextTranslationTerm,
};
