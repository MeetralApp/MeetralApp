//! Application error helpers — preserve full context in logs, short messages for IPC.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Error;

static PANIC_COUNT: AtomicU64 = AtomicU64::new(0);

/// Install a hook that logs panics via tracing and records them for diagnostics.
/// With `panic = "unwind"` a worker panic no longer tears down the process, so
/// without this hook the only signal would be a silently dead pipeline.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        PANIC_COUNT.fetch_add(1, Ordering::Relaxed);
        tracing::error!(panic = %info, "panic in worker thread");
    }));
}

pub fn panic_count() -> u64 {
    PANIC_COUNT.load(Ordering::Relaxed)
}

/// Log the full error chain and return a user-facing message for Tauri `Result<T, String>`.
pub fn log_and_stringify(err: impl Into<Error>) -> String {
    let err = err.into();
    tracing::error!("{err:#}");
    err.to_string()
}

/// Log with context label (e.g. operation name).
pub fn log_and_stringify_ctx(context: &str, err: impl Into<Error>) -> String {
    let err = err.into();
    tracing::error!("{context}: {err:#}");
    format!("{context}: {err}")
}
