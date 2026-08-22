use std::sync::mpsc;

use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
use tauri::WebviewWindow;

/// Best-effort on macOS — `set_content_protected` maps to NSWindowSharingNone.
/// ScreenCaptureKit on macOS 15+ may still capture the window.
pub fn apply_hide_from_capture(window: &WebviewWindow, hide: bool) -> Result<(), String> {
    window
        .set_content_protected(hide)
        .map_err(|e| e.to_string())
}

/// Toggle click-through via AppKit immediately (tao applies the same flag
/// asynchronously — too slow / racy for Cmd hold-to-interact).
pub fn set_ignore_cursor_events(window: &WebviewWindow, ignore: bool) -> Result<(), String> {
    with_ns_window(window, move |ns| {
        ns.setIgnoresMouseEvents(ignore);
    })?;
    // Keep tao WindowFlags in sync (best-effort; applied on the UI thread).
    let _ = window.set_ignore_cursor_events(ignore);
    Ok(())
}

/// Stabilize floating overlay behavior across Spaces / fullscreen apps.
pub fn harden_overlay_window(window: &WebviewWindow) -> Result<(), String> {
    with_ns_window(window, |ns| {
        // NSFloatingWindowLevel — matches always-on-top without status-level aggression.
        const NS_FLOATING_WINDOW_LEVEL: isize = 3;
        ns.setLevel(NS_FLOATING_WINDOW_LEVEL);
        ns.setHidesOnDeactivate(false);
        let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary;
        ns.setCollectionBehavior(behavior);
    })
}

fn with_ns_window(
    window: &WebviewWindow,
    f: impl FnOnce(&NSWindow) + Send + 'static,
) -> Result<(), String> {
    let win = window.clone();
    let run = move || -> Result<(), String> {
        let ptr = win.ns_window().map_err(|e| e.to_string())?;
        if ptr.is_null() {
            return Err("overlay ns_window is null".into());
        }
        // SAFETY: pointer from Tauri is a live NSWindow for this WebviewWindow.
        let ns = unsafe { &*(ptr as *const NSWindow) };
        f(ns);
        Ok(())
    };

    // setIgnoresMouseEvents / collectionBehavior must run on the main thread.
    if objc2::MainThreadMarker::new().is_some() {
        return run();
    }

    let (tx, rx) = mpsc::channel();
    window
        .run_on_main_thread(move || {
            let _ = tx.send(run());
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}
