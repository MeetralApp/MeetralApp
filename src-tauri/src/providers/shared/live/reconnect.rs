use std::hash::{BuildHasher, Hasher, RandomState};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tracing::{error, warn};

use super::session_error::SessionError;
use super::shared::{
    BridgeStatusEvent, BridgeStatusSender, ReconnectPolicy, MAX_RECONNECT_ATTEMPTS,
};

pub fn meeting_grade_backoff(attempts: u32) -> tokio::time::Duration {
    let extra = attempts.saturating_sub(MAX_RECONNECT_ATTEMPTS);
    let secs = 30u64.saturating_add((extra as u64).min(8) * 15);
    tokio::time::Duration::from_secs(secs.min(120))
}

/// Apply ±20% jitter to a reconnect delay.
///
/// Uses `RandomState` (OS entropy) so consecutive reconnects do not collapse
/// to the same −20% delay. A previous `Instant::now().elapsed()` on a fresh
/// Instant was ~0 ns, so `% 41` was always 0.
pub fn with_jitter(base: tokio::time::Duration) -> tokio::time::Duration {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u128(base.as_nanos());
    let pct = (hasher.finish() % 41) as i64 - 20;
    let ms = base.as_millis() as i64;
    let adjusted = ms + ms.saturating_mul(pct) / 100;
    tokio::time::Duration::from_millis(adjusted.max(0) as u64)
}

pub struct ReconnectLoopConfig<'a> {
    pub provider_label: &'a str,
    pub direction: &'a str,
    pub reconnect_policy: ReconnectPolicy,
}

pub enum ReconnectAction {
    BreakFatal(String),
    Retry { delay: tokio::time::Duration },
}

pub struct ReconnectState {
    pub attempts: u32,
    reconnect_delay: tokio::time::Duration,
}

impl Default for ReconnectState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReconnectState {
    pub fn new() -> Self {
        Self {
            attempts: 0,
            reconnect_delay: tokio::time::Duration::from_secs(1),
        }
    }

    pub fn reconnected(&self) -> bool {
        self.attempts > 0
    }
}

pub fn classify_session_error(
    err: anyhow::Error,
    config: &ReconnectLoopConfig<'_>,
    state: &mut ReconnectState,
) -> ReconnectAction {
    let message = format!("{err:#}");

    if let Some(session) = err.chain().find_map(|e| e.downcast_ref::<SessionError>()) {
        if session.is_fatal() {
            error!(
                "[{}:{}] fatal API error: {message}",
                config.provider_label, config.direction
            );
            return ReconnectAction::BreakFatal(message);
        }
    }

    state.attempts += 1;
    let should_fatal = match config.reconnect_policy {
        ReconnectPolicy::Limited => state.attempts >= MAX_RECONNECT_ATTEMPTS,
        ReconnectPolicy::MeetingGrade => false,
    };
    if should_fatal {
        error!(
            "[{}:{}] giving up after {} attempts: {message}",
            config.provider_label, config.direction, state.attempts
        );
        return ReconnectAction::BreakFatal(message);
    }

    warn!(
        "[{}:{}] session ended: {message}, reconnecting...",
        config.provider_label, config.direction
    );
    let delay = if config.reconnect_policy == ReconnectPolicy::MeetingGrade
        && state.attempts > MAX_RECONNECT_ATTEMPTS
    {
        meeting_grade_backoff(state.attempts)
    } else {
        state.reconnect_delay
    };
    if state.attempts <= MAX_RECONNECT_ATTEMPTS {
        state.reconnect_delay =
            (state.reconnect_delay * 2).min(tokio::time::Duration::from_secs(8));
    }

    ReconnectAction::Retry {
        delay: with_jitter(delay),
    }
}

pub async fn apply_retry(
    setup_complete: &Arc<AtomicBool>,
    status_tx: &BridgeStatusSender,
    direction: &str,
    attempts: u32,
    delay: tokio::time::Duration,
) {
    setup_complete.store(false, Ordering::SeqCst);
    crate::runtime::control_channel::try_send_control(
        status_tx,
        BridgeStatusEvent::Reconnecting {
            direction: direction.to_string(),
            attempt: attempts,
        },
        "bridge-status",
    );
    tokio::time::sleep(delay).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meeting_grade_backoff_starts_at_forty_five_seconds_after_quick_retries() {
        assert_eq!(
            meeting_grade_backoff(MAX_RECONNECT_ATTEMPTS + 1),
            tokio::time::Duration::from_secs(45)
        );
    }

    #[test]
    fn meeting_grade_backoff_at_boundary_is_thirty_seconds() {
        assert_eq!(
            meeting_grade_backoff(MAX_RECONNECT_ATTEMPTS),
            tokio::time::Duration::from_secs(30)
        );
    }

    #[test]
    fn meeting_grade_backoff_increases_with_extra_attempts() {
        assert_eq!(
            meeting_grade_backoff(MAX_RECONNECT_ATTEMPTS + 2),
            tokio::time::Duration::from_secs(60)
        );
    }

    #[test]
    fn meeting_grade_backoff_caps_at_two_minutes() {
        assert_eq!(
            meeting_grade_backoff(100),
            tokio::time::Duration::from_secs(120)
        );
    }

    #[test]
    fn classify_fatal_session_error_breaks_without_retry() {
        let config = ReconnectLoopConfig {
            provider_label: "Test",
            direction: "outbound",
            reconnect_policy: ReconnectPolicy::MeetingGrade,
        };
        let mut state = ReconnectState::new();
        let action = classify_session_error(
            SessionError::fatal("Gemini API error: invalid key"),
            &config,
            &mut state,
        );
        assert!(matches!(action, ReconnectAction::BreakFatal(_)));
        assert_eq!(state.attempts, 0);
    }

    #[test]
    fn classify_retryable_session_error_retries() {
        let config = ReconnectLoopConfig {
            provider_label: "Test",
            direction: "outbound",
            reconnect_policy: ReconnectPolicy::MeetingGrade,
        };
        let mut state = ReconnectState::new();
        let action = classify_session_error(
            SessionError::retryable("WebSocket closed"),
            &config,
            &mut state,
        );
        match action {
            ReconnectAction::Retry { delay } => {
                assert!(delay >= tokio::time::Duration::from_millis(800));
                assert!(delay <= tokio::time::Duration::from_millis(1200));
            }
            ReconnectAction::BreakFatal(m) => panic!("expected retry, got fatal: {m}"),
        }
        assert_eq!(state.attempts, 1);
    }

    #[test]
    fn classify_unknown_error_is_retryable() {
        let config = ReconnectLoopConfig {
            provider_label: "Test",
            direction: "inbound",
            reconnect_policy: ReconnectPolicy::MeetingGrade,
        };
        let mut state = ReconnectState::new();
        let action = classify_session_error(anyhow::anyhow!("something odd"), &config, &mut state);
        assert!(matches!(action, ReconnectAction::Retry { .. }));
        assert_eq!(state.attempts, 1);
    }

    #[test]
    fn classify_limited_policy_fatals_after_max_attempts() {
        let config = ReconnectLoopConfig {
            provider_label: "Test",
            direction: "outbound",
            reconnect_policy: ReconnectPolicy::Limited,
        };
        let mut state = ReconnectState::new();
        for _ in 0..(MAX_RECONNECT_ATTEMPTS - 1) {
            let action =
                classify_session_error(SessionError::retryable("closed"), &config, &mut state);
            assert!(matches!(action, ReconnectAction::Retry { .. }));
        }
        let action =
            classify_session_error(SessionError::retryable("closed again"), &config, &mut state);
        assert!(matches!(action, ReconnectAction::BreakFatal(_)));
        assert_eq!(state.attempts, MAX_RECONNECT_ATTEMPTS);
    }

    #[test]
    fn with_jitter_stays_within_twenty_percent() {
        let base = tokio::time::Duration::from_millis(1000);
        for _ in 0..32 {
            let j = with_jitter(base);
            assert!(j >= tokio::time::Duration::from_millis(800));
            assert!(j <= tokio::time::Duration::from_millis(1200));
        }
    }

    #[test]
    fn with_jitter_is_not_stuck_at_minus_twenty_percent() {
        let base = tokio::time::Duration::from_millis(1000);
        let samples: Vec<_> = (0..24).map(|_| with_jitter(base).as_millis()).collect();
        let distinct: std::collections::HashSet<_> = samples.iter().copied().collect();
        assert!(
            distinct.len() > 1,
            "expected reconnect jitter to vary, got {samples:?}"
        );
        assert!(
            samples.iter().any(|&ms| ms != 800),
            "jitter collapsed to −20% on every sample: {samples:?}"
        );
    }
}
