//! Transcript overlay window — Windows + macOS (best-effort capture exclusion).

mod clickthrough;
#[cfg(target_os = "macos")]
mod macos;
mod platform;
pub mod window;
#[cfg(windows)]
mod windows;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;

use crate::config::{AppConfig, OverlaySettings};
use crate::meeting::{ActiveMeetingId, MeetingStore};
use crate::runtime::engine::SharedEngine;

pub use window::{ensure_overlay_window, OVERLAY_LABEL};

static PREVIEW_MODE: AtomicBool = AtomicBool::new(false);

fn sync_click_through(app: &AppHandle, settings: &OverlaySettings) {
    clickthrough::set_desired(app, settings.click_through);
}

pub fn apply_settings(app: &AppHandle, settings: &OverlaySettings) -> Result<(), String> {
    let win = ensure_overlay_window(app)?;
    window::apply_geometry(&win, app, settings)?;
    platform::apply_hide_from_capture(&win, settings.hide_from_capture)?;

    if settings.enabled {
        let visible = win.is_visible().unwrap_or(false);
        if visible {
            // Already showing — do not re-show (that destabilized ignore on Windows).
            sync_click_through(app, settings);
        } else {
            show_inner(app, settings)?;
        }
    } else {
        PREVIEW_MODE.store(false, Ordering::Relaxed);
        clickthrough::clear(app);
        let _ = win.hide();
    }
    Ok(())
}

pub fn show(app: &AppHandle) -> Result<(), String> {
    // Only notify FE when leaving preview — a spurious `false` while already
    // live clears the overlay transcript tail (Rescue / Shift+K hotkeys call
    // show on an already-visible live overlay).
    if PREVIEW_MODE.swap(false, Ordering::Relaxed) {
        let _ = app.emit("overlay-preview-mode", false);
    }
    let settings = current_settings(app);
    show_inner(app, &settings)
}

pub fn hide(app: &AppHandle) -> Result<(), String> {
    if PREVIEW_MODE.swap(false, Ordering::Relaxed) {
        let _ = app.emit("overlay-preview-mode", false);
    }
    clickthrough::clear(app);
    let win = ensure_overlay_window(app)?;
    win.hide().map_err(|e| e.to_string())
}

pub fn toggle(app: &AppHandle) -> Result<(), String> {
    toggle_enabled(app).map(|_| ())
}

pub fn preview(app: &AppHandle) -> Result<(), String> {
    if has_current_session(app) {
        return Err("Overlay preview is unavailable while a current session is open".into());
    }
    PREVIEW_MODE.store(true, Ordering::Relaxed);
    let settings = current_settings(app);
    show_inner(app, &settings)?;
    // Emit after show so a freshly loaded overlay webview can receive it.
    // Preview only seeds mock transcript — click-through follows settings.
    let _ = app.emit("overlay-preview-mode", true);
    Ok(())
}

pub fn is_preview_mode() -> bool {
    PREVIEW_MODE.load(Ordering::Relaxed)
}

/// True when Library shows a Current session (live meeting), regardless of translate state.
fn has_current_session(app: &AppHandle) -> bool {
    if let Some(active) = app.try_state::<ActiveMeetingId>() {
        if let Ok(guard) = active.lock() {
            if guard.is_some() {
                return true;
            }
        }
    }
    if let Some(store) = app.try_state::<std::sync::Arc<MeetingStore>>() {
        matches!(store.get_live_meeting(), Ok(Some(_)))
    } else {
        false
    }
}

/// While true, click-through On still receives pointer events (open menus).
pub fn set_pointer_interactive(app: &AppHandle, interactive: bool) {
    clickthrough::set_hold_interactive(app, interactive);
}

/// Flip Show transcript overlay and apply immediately (hotkey / tray).
pub fn toggle_enabled(app: &AppHandle) -> Result<bool, String> {
    let store = app
        .try_state::<AsyncMutex<AppConfig>>()
        .ok_or_else(|| "config store missing".to_string())?;
    let Ok(mut guard) = store.try_lock() else {
        return Err("config store busy".to_string());
    };
    guard.overlay.enabled = !guard.overlay.enabled;
    let enabled = guard.overlay.enabled;
    let overlay = guard.overlay.clone();
    let merged = guard.clone();
    drop(guard);

    apply_settings(app, &overlay)?;
    crate::tray::set_overlay_menu_checked(app, enabled);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let save_cfg = merged.clone();
        let app_save = app_bg.clone();
        let _ =
            tokio::task::spawn_blocking(move || crate::config_store::save(&app_save, &save_cfg))
                .await;
        if let Some(engine) = app_bg.try_state::<SharedEngine>() {
            let _ = crate::app_state::sync_app_state_after_config_change(
                &app_bg,
                engine.inner(),
                &merged,
            )
            .await;
        }
    });

    Ok(enabled)
}

/// Flip click-through and apply immediately (for global hotkey / tray).
pub fn toggle_click_through(app: &AppHandle) -> Result<bool, String> {
    let store = app
        .try_state::<AsyncMutex<AppConfig>>()
        .ok_or_else(|| "config store missing".to_string())?;
    let Ok(mut guard) = store.try_lock() else {
        return Err("config store busy".to_string());
    };
    guard.overlay.click_through = !guard.overlay.click_through;
    let enabled = guard.overlay.click_through;
    let overlay = guard.overlay.clone();
    let merged = guard.clone();
    drop(guard);

    sync_click_through(app, &overlay);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let save_cfg = merged.clone();
        let app_save = app_bg.clone();
        let _ =
            tokio::task::spawn_blocking(move || crate::config_store::save(&app_save, &save_cfg))
                .await;
        if let Some(engine) = app_bg.try_state::<SharedEngine>() {
            let _ = crate::app_state::sync_app_state_after_config_change(
                &app_bg,
                engine.inner(),
                &merged,
            )
            .await;
        }
    });

    Ok(enabled)
}

fn show_inner(app: &AppHandle, settings: &OverlaySettings) -> Result<(), String> {
    let win = ensure_overlay_window(app)?;
    window::apply_geometry(&win, app, settings)?;
    platform::apply_hide_from_capture(&win, settings.hide_from_capture)?;
    platform::harden_overlay_window(&win);
    sync_click_through(app, settings);
    win.show().map_err(|e| e.to_string())?;
    // Re-apply after show() — avoid WDA_MONITOR black-box regression.
    platform::apply_hide_from_capture(&win, settings.hide_from_capture)?;
    platform::harden_overlay_window(&win);
    // Re-sync click-through after show — some platforms reset cursor flags.
    sync_click_through(app, settings);
    Ok(())
}

pub fn set_offset(app: &AppHandle, offset_x: f64, offset_y: f64) -> Result<(), String> {
    let store = app
        .try_state::<AsyncMutex<AppConfig>>()
        .ok_or_else(|| "config store missing".to_string())?;
    let Ok(mut guard) = store.try_lock() else {
        return Err("config store busy".to_string());
    };
    let scale = app
        .get_webview_window(OVERLAY_LABEL)
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0);
    let logical_x = offset_x / scale;
    let logical_y = offset_y / scale;
    let anchor = window::preset_anchor(app, &guard.overlay);
    guard.overlay.offset_x = logical_x - anchor.0;
    guard.overlay.offset_y = logical_y - anchor.1;
    let overlay = guard.overlay.clone();
    let for_disk = guard.clone();
    drop(guard);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || crate::config_store::save(&app_bg, &for_disk))
            .await;
    });

    if let Some(win) = app.get_webview_window(OVERLAY_LABEL) {
        window::apply_geometry(&win, app, &overlay)?;
    }
    Ok(())
}

/// Persist overlay size (logical px). Keeps the window's current top-left by
/// rebasing `offset_x/y` against the size-dependent preset anchor — otherwise
/// BottomCenter / *Right anchors shift with width/height and the window jumps.
pub fn set_size(app: &AppHandle, width: f64, height: f64) -> Result<(), String> {
    let store = app
        .try_state::<AsyncMutex<AppConfig>>()
        .ok_or_else(|| "config store missing".to_string())?;
    let Ok(mut guard) = store.try_lock() else {
        return Err("config store busy".to_string());
    };

    let win = app.get_webview_window(OVERLAY_LABEL);
    let current_pos = win.as_ref().and_then(|w| {
        let scale = w.scale_factor().ok()?;
        let pos = w.outer_position().ok()?;
        Some((pos.x as f64 / scale, pos.y as f64 / scale))
    });

    guard.overlay.width = width;
    guard.overlay.height = height;
    guard.overlay.normalize();

    if let Some((cx, cy)) = current_pos {
        let anchor = window::preset_anchor(app, &guard.overlay);
        guard.overlay.offset_x = cx - anchor.0;
        guard.overlay.offset_y = cy - anchor.1;
    }

    let overlay = guard.overlay.clone();
    let for_disk = guard.clone();
    drop(guard);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || crate::config_store::save(&app_bg, &for_disk))
            .await;
    });

    // Size only — position is already correct from the live drag; avoid
    // re-set_position which can still jitter from physical rounding.
    if let Some(win) = win {
        let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
            overlay.width,
            overlay.height,
        )));
    }
    Ok(())
}

pub fn on_pipeline_status(app: &AppHandle, any_session_active: bool) {
    // Real session supersedes Review overlay — drop mock transcript on the FE.
    if any_session_active && PREVIEW_MODE.swap(false, Ordering::Relaxed) {
        let _ = app.emit("overlay-preview-mode", false);
    }

    let settings = current_settings(app);
    if !settings.enabled || !settings.auto_show_with_session {
        return;
    }
    if any_session_active {
        let visible = app
            .get_webview_window(OVERLAY_LABEL)
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if visible {
            // Keep click-through owner stable — do not re-show.
            sync_click_through(app, &settings);
            return;
        }
        if let Err(e) = show_inner(app, &settings) {
            tracing::warn!("overlay auto-show failed: {e}");
        }
    }
    // Idle: do not auto-hide — user dismisses via hide control / tray.
}

fn current_settings(app: &AppHandle) -> OverlaySettings {
    if let Some(store) = app.try_state::<AsyncMutex<AppConfig>>() {
        store
            .try_lock()
            .map(|g| g.overlay.clone())
            .unwrap_or_default()
    } else {
        OverlaySettings::default()
    }
}
