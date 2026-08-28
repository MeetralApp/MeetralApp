//! Queue speakable units on the single xAI socket.
//!
//! The socket is sequential (`text.done` → wait `audio.done`). Extra units
//! wait in `pending`. After `text.clear`, stay in `Cancelling` until the server
//! acks so stale PCM is not treated as the next utterance.

use std::collections::VecDeque;

use super::pack::SpeakableUnit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Generating,
    Cancelling,
}

#[derive(Debug)]
pub struct SocketQueue {
    pending: VecDeque<SpeakableUnit>,
    phase: Phase,
}

impl SocketQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            phase: Phase::Idle,
        }
    }

    pub fn is_generating(&self) -> bool {
        matches!(self.phase, Phase::Generating)
    }

    pub fn push_units(&mut self, units: Vec<SpeakableUnit>) -> Option<SpeakableUnit> {
        for u in units {
            self.pending.push_back(u);
        }
        self.try_start()
    }

    pub fn on_audio_done(&mut self) -> Option<SpeakableUnit> {
        match self.phase {
            Phase::Generating | Phase::Cancelling => {
                self.phase = Phase::Idle;
                self.try_start()
            }
            Phase::Idle => None,
        }
    }

    pub fn on_audio_clear(&mut self) -> Option<SpeakableUnit> {
        if self.phase != Phase::Cancelling {
            return None;
        }
        self.phase = Phase::Idle;
        self.try_start()
    }

    /// Drop queued units. In-flight generation waits for `audio.done` / `audio.clear`.
    pub fn begin_reset(&mut self) {
        self.pending.clear();
        if self.phase == Phase::Generating {
            self.phase = Phase::Cancelling;
        }
    }

    fn try_start(&mut self) -> Option<SpeakableUnit> {
        if self.phase != Phase::Idle {
            return None;
        }
        let unit = self.pending.pop_front()?;
        self.phase = Phase::Generating;
        Some(unit)
    }
}

impl Default for SocketQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(text: &str) -> SpeakableUnit {
        SpeakableUnit { text: text.into() }
    }

    #[test]
    fn second_unit_waits_until_audio_done() {
        let mut q = SocketQueue::new();
        let first = q.push_units(vec![unit("one"), unit("two")]);
        assert_eq!(first.unwrap().text, "one");
        assert!(q.is_generating());
        assert_eq!(q.pending.len(), 1);
        let next = q
            .on_audio_done()
            .expect("queued unit starts after audio.done");
        assert_eq!(next.text, "two");
    }

    #[test]
    fn reset_does_not_assign_until_cancel_acks() {
        let mut q = SocketQueue::new();
        let _ = q.push_units(vec![unit("a")]);
        q.begin_reset();
        assert!(q.push_units(vec![unit("c")]).is_none());
        let next = q
            .on_audio_clear()
            .expect("pending unit starts after cancel ack");
        assert_eq!(next.text, "c");
    }

    #[test]
    fn trailing_audio_clear_does_not_finish_the_next_unit() {
        let mut q = SocketQueue::new();
        let _ = q.push_units(vec![unit("a")]);
        assert!(q.on_audio_done().is_none());
        let _ = q.push_units(vec![unit("b")]);
        assert!(q.on_audio_clear().is_none());
        assert!(q.is_generating());
    }
}
