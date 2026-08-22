use tauri::WebviewWindow;

pub fn apply_hide_from_capture(window: &WebviewWindow, hide: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        super::windows::apply_hide_from_capture(window, hide)
    }
    #[cfg(target_os = "macos")]
    {
        super::macos::apply_hide_from_capture(window, hide)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (window, hide);
        Ok(())
    }
}

/// `ignore == true` → clicks pass through the overlay.
pub fn set_ignore_cursor_events(window: &WebviewWindow, ignore: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        super::windows::set_ignore_cursor_events(window, ignore)
    }
    #[cfg(target_os = "macos")]
    {
        super::macos::set_ignore_cursor_events(window, ignore)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        window
            .set_ignore_cursor_events(ignore)
            .map_err(|e| e.to_string())
    }
}

/// macOS: floating level + Space/fullscreen collection behavior. No-op elsewhere.
pub fn harden_overlay_window(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = super::macos::harden_overlay_window(window) {
            tracing::warn!("overlay harden_overlay_window failed: {e}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }
}
