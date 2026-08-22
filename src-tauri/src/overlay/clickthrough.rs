//! Click-through: when desired, ignore cursor unless Ctrl (Windows) / Cmd (macOS) is held.
//! Single owner of `set_ignore_cursor_events` for the overlay window.
//!
//! Preview mode (mock transcript) does **not** bypass this — Open window must
//! behave like the live overlay so click-through can be verified.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use super::platform;
use super::OVERLAY_LABEL;

static DESIRED: AtomicBool = AtomicBool::new(false);
static LAST_IGNORE: AtomicBool = AtomicBool::new(false);
static HAS_LAST_IGNORE: AtomicBool = AtomicBool::new(false);
static LOOP_STARTED: AtomicBool = AtomicBool::new(false);
/// FE holds this while a dropdown/menu is open so clicks still hit the overlay
/// even if the hold modifier is released mid-selection (click-through On).
static HOLD_INTERACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_desired(app: &AppHandle, desired: bool) {
    ensure_loop(app);
    DESIRED.store(desired, Ordering::Relaxed);
    HAS_LAST_IGNORE.store(false, Ordering::Relaxed);
    if !desired {
        HOLD_INTERACTIVE.store(false, Ordering::Relaxed);
        apply_ignore(app, false);
        return;
    }
    tick(app);
}

/// Keep the overlay interactive while a popover/menu is open (click-through On).
pub fn set_hold_interactive(app: &AppHandle, hold: bool) {
    ensure_loop(app);
    HOLD_INTERACTIVE.store(hold, Ordering::Relaxed);
    HAS_LAST_IGNORE.store(false, Ordering::Relaxed);
    if !DESIRED.load(Ordering::Relaxed) {
        if !hold {
            apply_ignore(app, false);
        }
        return;
    }
    tick(app);
}

/// Force interactive (ignore=false) and clear desired — used on hide/disable.
pub fn clear(app: &AppHandle) {
    DESIRED.store(false, Ordering::Relaxed);
    HOLD_INTERACTIVE.store(false, Ordering::Relaxed);
    HAS_LAST_IGNORE.store(false, Ordering::Relaxed);
    apply_ignore(app, false);
}

fn ensure_loop(app: &AppHandle) {
    if LOOP_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            // 80ms is enough for modifier hold detection; lower rates cut idle CPU
            // on long meetings with click-through enabled.
            tokio::time::sleep(Duration::from_millis(80)).await;
            if !DESIRED.load(Ordering::Relaxed) && !HOLD_INTERACTIVE.load(Ordering::Relaxed) {
                continue;
            }
            tick(&app);
        }
    });
}

fn tick(app: &AppHandle) {
    let desired = DESIRED.load(Ordering::Relaxed);
    if !desired {
        return;
    }
    let Some(win) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    if !win.is_visible().unwrap_or(false) {
        return;
    }

    let modifier = platform_modifier_pressed();
    let hold = HOLD_INTERACTIVE.load(Ordering::Relaxed);
    // Hold Ctrl (Windows) / Cmd (macOS) → interact; release → pass-through again.
    // Menu/popover can pin interactive via HOLD_INTERACTIVE.
    let ignore = desired && !modifier && !hold;
    apply_ignore_if_changed(app, ignore);
}

fn apply_ignore_if_changed(app: &AppHandle, ignore: bool) {
    if HAS_LAST_IGNORE.load(Ordering::Relaxed) && LAST_IGNORE.load(Ordering::Relaxed) == ignore {
        return;
    }
    apply_ignore(app, ignore);
}

fn apply_ignore(app: &AppHandle, ignore: bool) {
    let Some(win) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let was_ignoring =
        HAS_LAST_IGNORE.load(Ordering::Relaxed) && LAST_IGNORE.load(Ordering::Relaxed);
    match platform::set_ignore_cursor_events(&win, ignore) {
        Ok(()) => {
            LAST_IGNORE.store(ignore, Ordering::Relaxed);
            HAS_LAST_IGNORE.store(true, Ordering::Relaxed);
            tracing::debug!(ignore, "overlay cursor ignore updated");
            // Pass-through again: FE never gets pointerleave (cursor ignored), so
            // dismiss hover chrome (tooltips) that would otherwise stick open.
            if ignore && !was_ignoring {
                let _ = app.emit("overlay-pass-through", true);
            }
        }
        Err(e) => {
            tracing::warn!("overlay set_ignore_cursor_events({ignore}) failed: {e}");
        }
    }
}

fn platform_modifier_pressed() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
        // SAFETY: GetAsyncKeyState is a well-defined Win32 query with no side effects.
        unsafe { GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000 != 0 }
    }
    #[cfg(target_os = "macos")]
    {
        // kCGEventSourceStateCombinedSessionState = 0
        // kVK_Command = 0x37, kVK_RightCommand = 0x36
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventSourceKeyState(state_id: u32, key: u16) -> bool;
        }
        // SAFETY: CoreGraphics key-state query; keycodes are HIToolbox constants.
        unsafe { CGEventSourceKeyState(0, 0x37) || CGEventSourceKeyState(0, 0x36) }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        false
    }
}
