use std::path::PathBuf;

use anyhow::Result;
use tauri::AppHandle;

pub fn meeting_db_path(app: &AppHandle) -> Result<PathBuf> {
    let mut path = crate::app_data::ensure_dir(app)?;
    path.push("meetings.db");
    Ok(path)
}

/// Default recordings root: `{app_data}/recordings`.
pub fn default_recordings_dir(app: &AppHandle) -> Result<PathBuf> {
    let mut path = crate::app_data::dir(app)?;
    path.push("recordings");
    Ok(path)
}

/// Resolve configured save folder; empty/whitespace → default app recordings dir.
pub fn resolve_recordings_base(app: &AppHandle, configured: &str) -> Result<PathBuf> {
    let trimmed = configured.trim();
    if trimmed.is_empty() {
        default_recordings_dir(app)
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

pub fn meeting_recordings_dir(base: &std::path::Path, meeting_id: &str) -> PathBuf {
    base.join(meeting_id)
}

pub fn chunk_path(base: &std::path::Path, meeting_id: &str, direction: &str, seq: u32) -> PathBuf {
    meeting_recordings_dir(base, meeting_id)
        .join(direction)
        .join(format!("{seq}.opus"))
}

pub fn format_meeting_title(now_ms: i64) -> String {
    use chrono::{Datelike, Local, TimeZone};
    let secs = (now_ms / 1000).max(0);
    let nanos = ((now_ms % 1000).max(0) as u32) * 1_000_000;
    let dt = Local
        .timestamp_opt(secs, nanos)
        .single()
        .unwrap_or_else(Local::now);
    let now = Local::now();
    let same_day = dt.date_naive() == now.date_naive();
    let same_year = dt.year() == now.year();
    if same_day {
        dt.format("%H:%M").to_string()
    } else if same_year {
        dt.format("%b %d, %H:%M").to_string()
    } else {
        dt.format("%b %d, %Y, %H:%M").to_string()
    }
}

pub fn wall_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_path_layout() {
        let base = PathBuf::from("/data/recordings");
        let p = chunk_path(&base, "meet-1", "inbound", 3);
        assert_eq!(p, PathBuf::from("/data/recordings/meet-1/inbound/3.opus"));
    }
}
