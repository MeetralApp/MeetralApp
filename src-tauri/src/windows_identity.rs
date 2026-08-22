//! Windows sparse-package identity (Task Manager grouping / notifications).
//!
//! NSIS registers at install via `installer-hooks.nsh`. MSI historically did not,
//! so a freshly installed MSI process can run without package identity even when
//! `Meetral-sparse.msix` sits next to the exe. This module closes that gap:
//! register once at startup, then relaunch so the new process inherits identity.

use std::path::{Path, PathBuf};
use std::process::Command;

const RELAUNCH_ENV: &str = "MEETRAL_IDENTITY_RELAUNCH";

/// Returns `true` when this process should exit immediately (a replacement was spawned).
pub fn ensure_before_start() -> bool {
    if has_package_identity() {
        return false;
    }

    let Some(install_dir) = current_install_dir() else {
        return false;
    };
    let msix = install_dir.join("Meetral-sparse.msix");
    let script = install_dir.join("Register-MeetralIdentity.ps1");
    if !msix.is_file() || !script.is_file() {
        return false;
    }

    if std::env::var_os(RELAUNCH_ENV).is_some() {
        tracing::warn!(
            path = %install_dir.display(),
            "sparse package present but process still has no package identity after relaunch"
        );
        return false;
    }

    tracing::info!(
        path = %install_dir.display(),
        "registering sparse package identity (Task Manager grouping)"
    );
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .arg("-InstallDir")
        .arg(&install_dir)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            tracing::warn!(?s, "sparse identity registration exited non-zero");
            return false;
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to spawn sparse identity registration");
            return false;
        }
    }

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    match Command::new(&exe).env(RELAUNCH_ENV, "1").spawn() {
        Ok(_) => {
            tracing::info!("relaunching with package identity");
            true
        }
        Err(e) => {
            tracing::warn!(error = %e, "registered identity but failed to relaunch");
            false
        }
    }
}

fn current_install_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(Path::to_path_buf)
}

fn has_package_identity() -> bool {
    // GetCurrentPackageFullName returns APPMODEL_ERROR_NO_PACKAGE (15700) when
    // the process is not running with package identity.
    const APPMODEL_ERROR_NO_PACKAGE: i32 = 15700;
    const ERROR_INSUFFICIENT_BUFFER: i32 = 122;

    let mut len: u32 = 0;
    // SAFETY: First call probes length; null buffer is required by the API.
    let hr = unsafe { GetCurrentPackageFullName(&mut len, std::ptr::null_mut()) };
    if hr == APPMODEL_ERROR_NO_PACKAGE {
        return false;
    }
    if hr != ERROR_INSUFFICIENT_BUFFER && hr != 0 {
        return false;
    }
    if len == 0 {
        return false;
    }
    let mut buf = vec![0u16; len as usize];
    // SAFETY: `buf` is writable for `len` UTF-16 code units as reported by the probe.
    let hr = unsafe { GetCurrentPackageFullName(&mut len, buf.as_mut_ptr()) };
    hr == 0
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentPackageFullName(packageFullNameLength: *mut u32, packageFullName: *mut u16)
        -> i32;
}
