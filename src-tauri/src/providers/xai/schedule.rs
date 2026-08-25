//! Assign speakable units to free xAI sockets (max two). Extra units queue.
//!
//! Each socket is sequential (`text.done` → wait `audio.done`). A second socket
//! starts unit N+1 while N is still generating. After `text.clear`, the socket
//! stays in `Cancelling` until the server acks so stale PCM is not attributed
//! to the next unit.

use std::collections::VecDeque;

use super::pack::SpeakableUnit;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SlotPhase {
    Idle,
    Generating { unit: SpeakableUnit },
    Cancelling,
    Dead,
}

#[derive(Debug)]
pub enum SlotFinish {
    Completed {
        seq: u64,
        next: Option<(usize, SpeakableUnit)>,
    },
    Cancelled {
        next: Option<(usize, SpeakableUnit)>,
    },
    Ignored,
}

#[derive(Debug)]
pub struct SlotAssigner {
    pending: VecDeque<SpeakableUnit>,
    phase: [SlotPhase; 2],
}

impl SlotAssigner {
    pub fn new(slot_count: usize) -> Self {
        let mut phase = [SlotPhase::Idle, SlotPhase::Idle];
        if slot_count < 2 {
            phase[1] = SlotPhase::Dead;
        }
        Self {
            pending: VecDeque::new(),
            phase,
        }
    }

    pub fn slot_seq(&self, slot: usize) -> Option<u64> {
        match self.phase.get(slot)? {
            SlotPhase::Generating { unit } => Some(unit.seq),
            _ => None,
        }
    }

    pub fn live_slots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..2).filter(|&i| !matches!(self.phase[i], SlotPhase::Dead))
    }

    pub fn all_dead(&self) -> bool {
        self.phase.iter().all(|p| matches!(p, SlotPhase::Dead))
    }

    pub fn push_units(&mut self, units: Vec<SpeakableUnit>) -> Vec<(usize, SpeakableUnit)> {
        for u in units {
            self.pending.push_back(u);
        }
        self.drain_assign()
    }

    pub fn on_audio_done(&mut self, slot: usize) -> SlotFinish {
        self.on_audio_finished(slot)
    }

    pub fn on_audio_clear(&mut self, slot: usize) -> SlotFinish {
        if slot < 2 && matches!(self.phase[slot], SlotPhase::Cancelling) {
            self.on_audio_finished(slot)
        } else {
            SlotFinish::Ignored
        }
    }

    fn on_audio_finished(&mut self, slot: usize) -> SlotFinish {
        if slot >= 2 {
            return SlotFinish::Ignored;
        }
        let generating_seq = match &self.phase[slot] {
            SlotPhase::Generating { unit } => Some(unit.seq),
            SlotPhase::Cancelling => None,
            _ => return SlotFinish::Ignored,
        };
        self.phase[slot] = SlotPhase::Idle;
        if let Some(seq) = generating_seq {
            SlotFinish::Completed {
                seq,
                next: self.try_assign(),
            }
        } else {
            SlotFinish::Cancelled {
                next: self.try_assign(),
            }
        }
    }

    /// Drop queued units. In-flight sockets wait for `audio.done` / `audio.clear`.
    pub fn begin_reset(&mut self) {
        self.pending.clear();
        for phase in &mut self.phase {
            if matches!(phase, SlotPhase::Generating { .. }) {
                *phase = SlotPhase::Cancelling;
            }
        }
    }

    /// Socket died. Re-queue an in-flight unit onto the remaining socket.
    pub fn on_socket_dead(&mut self, slot: usize) -> Vec<(usize, SpeakableUnit)> {
        let stolen = match std::mem::replace(&mut self.phase[slot], SlotPhase::Dead) {
            SlotPhase::Generating { unit } => Some(unit),
            _ => None,
        };
        if let Some(unit) = stolen {
            self.pending.push_front(unit);
        }
        self.drain_assign()
    }

    fn drain_assign(&mut self) -> Vec<(usize, SpeakableUnit)> {
        let mut assigned = Vec::new();
        while let Some(pair) = self.try_assign() {
            assigned.push(pair);
        }
        assigned
    }

    fn try_assign(&mut self) -> Option<(usize, SpeakableUnit)> {
        if self.pending.is_empty() {
            return None;
        }
        let slot = (0..2).find(|&i| matches!(self.phase[i], SlotPhase::Idle))?;
        let unit = self.pending.pop_front()?;
        self.phase[slot] = SlotPhase::Generating { unit: unit.clone() };
        Some((slot, unit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(seq: u64, text: &str) -> SpeakableUnit {
        SpeakableUnit {
            seq,
            text: text.into(),
        }
    }

    #[test]
    fn two_units_fill_two_sockets() {
        let mut a = SlotAssigner::new(2);
        let got = a.push_units(vec![unit(0, "one"), unit(1, "two")]);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, 0);
        assert_eq!(got[1].0, 1);
        assert_eq!(a.slot_seq(0), Some(0));
        assert_eq!(a.slot_seq(1), Some(1));
    }

    #[test]
    fn third_unit_queues_until_socket_frees() {
        let mut a = SlotAssigner::new(2);
        let _ = a.push_units(vec![unit(0, "a"), unit(1, "b"), unit(2, "c")]);
        assert_eq!(a.pending.len(), 1);
        match a.on_audio_done(0) {
            SlotFinish::Completed { seq, next } => {
                assert_eq!(seq, 0);
                let next = next.expect("queued unit should start on the freed socket");
                assert_eq!(next.0, 0);
                assert_eq!(next.1.seq, 2);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(a.pending.is_empty());
    }

    #[test]
    fn single_socket_stays_sequential() {
        let mut a = SlotAssigner::new(1);
        let first = a.push_units(vec![unit(0, "a"), unit(1, "b")]);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1.seq, 0);
        match a.on_audio_done(0) {
            SlotFinish::Completed { next, .. } => {
                assert_eq!(next.unwrap().1.seq, 1);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reset_does_not_assign_until_cancel_acks() {
        let mut a = SlotAssigner::new(2);
        let _ = a.push_units(vec![unit(0, "a"), unit(1, "b")]);
        a.begin_reset();
        assert!(a.push_units(vec![unit(2, "c")]).is_empty());
        match a.on_audio_clear(0) {
            SlotFinish::Cancelled { next } => {
                let next = next.expect("pending unit starts after cancel ack");
                assert_eq!(next.0, 0);
                assert_eq!(next.1.seq, 2);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(a.slot_seq(1).is_none());
    }

    #[test]
    fn dead_socket_replays_inflight_on_the_other() {
        let mut a = SlotAssigner::new(2);
        let _ = a.push_units(vec![unit(0, "a"), unit(1, "b")]);
        let replayed = a.on_socket_dead(0);
        assert_eq!(replayed.len(), 0, "slot 1 is still busy");
        match a.on_audio_done(1) {
            SlotFinish::Completed { next, .. } => {
                assert_eq!(next.unwrap().1.seq, 0);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn trailing_audio_clear_does_not_finish_the_next_unit() {
        let mut a = SlotAssigner::new(2);
        let _ = a.push_units(vec![unit(0, "a")]);
        match a.on_audio_done(0) {
            SlotFinish::Completed { next, .. } => assert!(next.is_none()),
            other => panic!("unexpected {other:?}"),
        }
        let _ = a.push_units(vec![unit(1, "b")]);
        assert!(matches!(a.on_audio_clear(0), SlotFinish::Ignored));
        assert_eq!(a.slot_seq(0), Some(1));
    }
}
