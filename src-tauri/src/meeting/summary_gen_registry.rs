//! In-memory per-meeting summary generation state. The generate command holds a
//! [`SummaryGenGuard`] for the whole run; `Drop` clears the entry on every exit
//! path (success / error / panic-drop), so the map cannot leak.
//!
//! Purpose is **visibility + integrity within the session**: the FE restores
//! "generation in flight" UI after remount via `get_summary_generation_status`,
//! and a second generate for the same meeting is rejected instead of running a
//! duplicate LLM job (last-writer-wins on `meeting_summary`). App restart loses
//! in-flight jobs — durable resume is deliberately deferred.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::lock_poison_recover;

/// Invoke error string when a summary is already being generated. The FE maps
/// this exact prefix to a friendly inline message.
pub const SUMMARY_ALREADY_RUNNING: &str = "summary_already_running";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryGenState {
    pub phase: String,
    pub current: u32,
    pub total: u32,
    pub started_at_ms: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartError {
    AlreadyRunning,
}

#[derive(Debug, Default, Clone)]
pub struct SummaryGenerationRegistry {
    inner: Arc<Mutex<HashMap<String, SummaryGenState>>>,
}

impl SummaryGenerationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim a generation slot for the meeting. Second concurrent claim is
    /// rejected; the returned guard clears the entry on drop.
    pub fn try_start(
        &self,
        meeting_id: &str,
        started_at_ms: i64,
    ) -> Result<SummaryGenGuard, StartError> {
        let mut map = lock_poison_recover(&self.inner, "summary generation registry");
        if map.contains_key(meeting_id) {
            return Err(StartError::AlreadyRunning);
        }
        map.insert(
            meeting_id.to_string(),
            SummaryGenState {
                // Pre-first-progress state — FE label falls back to "Working…".
                phase: "starting".to_string(),
                current: 0,
                total: 0,
                started_at_ms,
            },
        );
        drop(map);
        Ok(SummaryGenGuard {
            registry: self.clone(),
            meeting_id: meeting_id.to_string(),
        })
    }

    /// Update the visible phase for a running generation. No-op when the
    /// meeting has no entry (e.g. status queried after completion).
    pub fn progress(&self, meeting_id: &str, phase: &str, current: u32, total: u32) {
        let mut map = lock_poison_recover(&self.inner, "summary generation registry");
        if let Some(state) = map.get_mut(meeting_id) {
            state.phase = phase.to_string();
            state.current = current;
            state.total = total;
        }
    }

    pub fn status(&self, meeting_id: &str) -> Option<SummaryGenState> {
        lock_poison_recover(&self.inner, "summary generation registry")
            .get(meeting_id)
            .cloned()
    }

    fn finish(&self, meeting_id: &str) {
        lock_poison_recover(&self.inner, "summary generation registry").remove(meeting_id);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        lock_poison_recover(&self.inner, "summary generation registry").len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Clears the registry entry on drop (every exit path).
#[derive(Debug)]
pub struct SummaryGenGuard {
    registry: SummaryGenerationRegistry,
    meeting_id: String,
}

impl Drop for SummaryGenGuard {
    fn drop(&mut self) {
        self.registry.finish(&self.meeting_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_start_rejects_second_concurrent_run() {
        let registry = SummaryGenerationRegistry::new();
        let guard = registry.try_start("m1", 100).unwrap();
        assert_eq!(
            registry.try_start("m1", 200).unwrap_err(),
            StartError::AlreadyRunning
        );
        // A different meeting is unaffected.
        let other = registry.try_start("m2", 200).unwrap();
        assert_eq!(registry.len(), 2);
        drop(other);
        drop(guard);
    }

    #[test]
    fn guard_drop_clears_entry_and_frees_the_slot() {
        let registry = SummaryGenerationRegistry::new();
        {
            let _guard = registry.try_start("m1", 100).unwrap();
            assert_eq!(registry.len(), 1);
        }
        assert_eq!(registry.len(), 0);
        assert!(registry.try_start("m1", 300).is_ok());
    }

    #[test]
    fn progress_updates_only_existing_entries() {
        let registry = SummaryGenerationRegistry::new();
        // No entry → no-op, no panic.
        registry.progress("ghost", "chunk", 1, 5);
        assert!(registry.status("ghost").is_none());

        let _guard = registry.try_start("m1", 100).unwrap();
        registry.progress("m1", "chunk", 2, 5);
        let state = registry.status("m1").unwrap();
        assert_eq!(
            state,
            SummaryGenState {
                phase: "chunk".to_string(),
                current: 2,
                total: 5,
                started_at_ms: 100,
            }
        );
    }
}
