use std::path::PathBuf;
use std::sync::Mutex;

use tauri::AppHandle;
use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Load optional `.env` from the repo root (and cwd) before logging / env checks.
/// Only in debug builds — release never reads `.env` (see V1 config policy).
#[cfg(debug_assertions)]
pub fn load_dev_env_files() {
    let root_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env");
    if root_env.is_file() {
        if let Err(e) = dotenvy::from_path(&root_env) {
            eprintln!("failed to load {}: {e}", root_env.display());
        }
    }
    let _ = dotenvy::dotenv();
}

#[cfg(not(debug_assertions))]
pub fn load_dev_env_files() {}

/// Owns the non-blocking log writer guard for the app lifetime. Wrapped in a
/// `Mutex` so it satisfies Tauri's `Send + Sync` state bound and so graceful
/// shutdown can drop the guard to flush buffered logs before the process exits
/// (replacing the previous `std::mem::forget`, which leaked the worker and lost
/// the final log lines).
pub struct LogGuard(Mutex<Option<WorkerGuard>>);

impl LogGuard {
    /// Flush and stop the background log writer. Safe to call multiple times.
    pub fn flush_and_close(&self) {
        if let Ok(mut guard) = self.0.lock() {
            // Dropping the WorkerGuard flushes any buffered log lines.
            guard.take();
        }
    }
}

pub fn init(app: &AppHandle) -> Option<LogGuard> {
    let log_dir = match app_log_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("failed to resolve log directory: {e:#}");
            init_console_only();
            return None;
        }
    };

    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("failed to create log directory {}: {e}", log_dir.display());
        init_console_only();
        return None;
    }

    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    tracing::info!("logging initialized at {}", log_dir.display());
    Some(LogGuard(Mutex::new(Some(guard))))
}

fn app_log_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let mut dir = app.path().app_data_dir()?;
    dir.push("logs");
    Ok(dir)
}

fn init_console_only() {
    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
}
