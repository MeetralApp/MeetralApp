//! Shared opt-in debug logging for local development.
//!
//! Set `DEBUG_MODE=1` (or `true`) in the project `.env` (repo root) or process
//! environment when running a **debug** build (`cargo tauri dev`).
//! Release builds never enable these traces.
//!
//! Use [`debug_log!`] for LLM / pipeline traces (and any other
//! opt-in diagnostic). Voice/STT helpers live under `voice::shared::debug`.

#[cfg(debug_assertions)]
const ENV_DEBUG_MODE: &str = "DEBUG_MODE";

/// `true` only in debug builds when `DEBUG_MODE` is `1` or `true`.
pub fn enabled() -> bool {
    #[cfg(not(debug_assertions))]
    {
        false
    }
    #[cfg(debug_assertions)]
    {
        matches!(
            std::env::var(ENV_DEBUG_MODE).as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        )
    }
}

/// Opt-in debug log gated by [`enabled`]. Emits at `info` with
/// `target: "debug_mode"` so lines are greppable and visible under the
/// default `EnvFilter` (`info`). Format args are only evaluated when enabled.
///
/// Prefer a short flow prefix in the message (`summary`, `live`, `tts`, …).
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::debug_mode::enabled() {
            ::tracing::info!(target: "debug_mode", $($arg)*);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_is_callable() {
        let _ = enabled();
    }

    #[test]
    fn debug_log_macro_compiles() {
        debug_log!("debug_mode smoke");
        debug_log!(value = 1u32, "debug_mode smoke with field");
    }
}
