use crate::runtime::engine::AudioConnectionState;

/// Audio device path lifecycle for one pipeline column (outbound or inbound).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AudioPathPhase {
    #[default]
    Ok,
    Reconnecting {
        attempt: u32,
        next_action_ms: Option<u64>,
    },
    Lost {
        since_ms: u64,
    },
}

impl AudioPathPhase {
    pub fn connection_state(&self) -> AudioConnectionState {
        match self {
            Self::Ok => AudioConnectionState::Ok,
            Self::Reconnecting { .. } => AudioConnectionState::Reconnecting,
            Self::Lost { .. } => AudioConnectionState::Lost,
        }
    }

    pub fn reconnect_attempt(&self) -> Option<u32> {
        match self {
            Self::Reconnecting { attempt, .. } => Some(*attempt),
            _ => None,
        }
    }

    pub fn next_action_ms(&self) -> Option<u64> {
        match self {
            Self::Reconnecting { next_action_ms, .. } => *next_action_ms,
            _ => None,
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }

    pub fn is_reconnecting(&self) -> bool {
        matches!(self, Self::Reconnecting { .. })
    }

    pub fn is_lost(&self) -> bool {
        matches!(self, Self::Lost { .. })
    }

    pub fn is_fault(&self) -> bool {
        self.is_reconnecting() || self.is_lost()
    }

    pub fn is_degraded(&self) -> bool {
        !self.is_ok()
    }

    pub fn clear_to_ok(&mut self) {
        *self = Self::Ok;
    }

    pub fn begin_reconnect(&mut self, attempt: u32) {
        *self = Self::Reconnecting {
            attempt,
            next_action_ms: None,
        };
    }

    pub fn set_next_action_ms(&mut self, deadline_ms: u64) {
        if let Self::Reconnecting { next_action_ms, .. } = self {
            *next_action_ms = Some(deadline_ms);
        }
    }

    pub fn mark_lost(&mut self, since_ms: u64) {
        *self = Self::Lost { since_ms };
    }

    /// Whether a scheduled reconnect / confirm window has elapsed.
    pub fn next_action_due(&self, now_ms: u64) -> bool {
        match self.next_action_ms() {
            Some(deadline) => now_ms >= deadline,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_can_clear_to_ok() {
        let mut phase = AudioPathPhase::Lost { since_ms: 100 };
        phase.clear_to_ok();
        assert!(phase.is_ok());
    }

    #[test]
    fn reconnecting_tracks_attempt_and_deadline() {
        let mut phase = AudioPathPhase::Ok;
        phase.begin_reconnect(3);
        phase.set_next_action_ms(5000);
        assert_eq!(phase.reconnect_attempt(), Some(3));
        assert!(!phase.next_action_due(4999));
        assert!(phase.next_action_due(5000));
    }
}
