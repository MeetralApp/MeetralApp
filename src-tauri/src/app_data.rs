//! Durable application data directory.
//!
//! On Windows, sparse/MSIX package identity virtualizes `%APPDATA%` into
//! `Packages\<pfn>\LocalCache\Roaming\...`, which Windows deletes when the
//! package is removed. `KF_FLAG_NO_PACKAGE_REDIRECTION` only returns the real
//! path string — `CreateFile` to that path is still redirected unless the
//! process uses a `\\?\` (verbatim) prefix. This module resolves
//! `%APPDATA%\<identifier>` and prefixes `\\?\` on Windows so config, the
//! meeting library, recordings, and logs land in real roaming AppData.
//! macOS keeps Tauri's Application Support path.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tauri::AppHandle;
use tauri::Manager;

pub fn dir(app: &AppHandle) -> Result<PathBuf> {
    Ok(for_fs(logical_dir(app)?))
}

pub fn ensure_dir(app: &AppHandle) -> Result<PathBuf> {
    let path = dir(app)?;
    std::fs::create_dir_all(&path).context("failed to create app data directory")?;
    Ok(path)
}

/// Win32 `\\?\` prefix stripped so folder pickers / UI can show a normal path.
pub fn for_display(path: &Path) -> PathBuf {
    strip_verbatim_prefix(path)
}

fn logical_dir(app: &AppHandle) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(roaming) = roaming_app_data_unvirtualized() {
            return Ok(roaming.join(app.config().identifier.as_str()));
        }
        if let Some(roaming) = roaming_app_data_from_userprofile() {
            return Ok(roaming.join(app.config().identifier.as_str()));
        }
    }
    app.path()
        .app_data_dir()
        .context("failed to resolve app data directory")
}

fn for_fs(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        verbatim_prefix(path)
    }
    #[cfg(not(windows))]
    {
        path
    }
}

#[cfg(windows)]
fn verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\") {
        return path;
    }
    PathBuf::from(format!(r"\\?\{}", s.trim_end_matches(['\\', '/'])))
}

fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

#[cfg(windows)]
fn roaming_app_data_unvirtualized() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_NO_PACKAGE_REDIRECTION,
    };

    unsafe {
        let pwstr = SHGetKnownFolderPath(
            &FOLDERID_RoamingAppData,
            KF_FLAG_NO_PACKAGE_REDIRECTION,
            None,
        )
        .ok()?;
        if pwstr.is_null() {
            return None;
        }
        let path = PathBuf::from(OsString::from_wide(pwstr.as_wide()));
        CoTaskMemFree(Some(pwstr.0.cast()));
        Some(path)
    }
}

#[cfg(windows)]
fn roaming_app_data_from_userprofile() -> Option<PathBuf> {
    let profile = std::env::var_os("USERPROFILE")?;
    let path = PathBuf::from(profile).join("AppData").join("Roaming");
    path.is_dir().then_some(path)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn verbatim_prefix_round_trips_for_display() {
        let logical = PathBuf::from(r"C:\Users\nhantruong\AppData\Roaming\com.meetral.desktop");
        let io = verbatim_prefix(logical.clone());
        assert_eq!(
            io,
            PathBuf::from(r"\\?\C:\Users\nhantruong\AppData\Roaming\com.meetral.desktop")
        );
        assert_eq!(strip_verbatim_prefix(&io), logical);
        assert_eq!(verbatim_prefix(io.clone()), io);
    }

    #[test]
    fn unvirtualized_roaming_is_not_package_localcache() {
        let path = roaming_app_data_unvirtualized()
            .or_else(roaming_app_data_from_userprofile)
            .expect("Roaming AppData");
        let lower = path.to_string_lossy().to_ascii_lowercase();
        assert!(
            !lower.contains(r"\packages\") && !lower.contains(r"\localcache\"),
            "expected real roaming AppData, got {}",
            path.display()
        );
        assert!(
            lower.ends_with(r"\appdata\roaming"),
            "expected ...\\AppData\\Roaming, got {}",
            path.display()
        );
    }
}
