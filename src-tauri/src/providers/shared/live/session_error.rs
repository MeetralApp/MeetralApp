//! Typed live-session errors for reconnect classification.

use std::fmt;

/// Fatal vs retryable session failure — constructed at provider bridge sites.
///
/// [`crate::providers::shared::live::classify_session_error`] downcasts this instead of
/// matching string prefixes.
#[derive(Debug, Clone)]
pub enum SessionError {
    Fatal(String),
    Retryable(String),
}

impl SessionError {
    pub fn fatal(msg: impl Into<String>) -> anyhow::Error {
        anyhow::Error::new(Self::Fatal(msg.into()))
    }

    pub fn retryable(msg: impl Into<String>) -> anyhow::Error {
        anyhow::Error::new(Self::Retryable(msg.into()))
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Fatal(m) | Self::Retryable(m) => m.as_str(),
        }
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for SessionError {}
