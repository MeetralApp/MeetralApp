use tauri::WebviewWindow;

pub fn apply_hide_from_capture(window: &WebviewWindow, hide: bool) -> Result<(), String> {
    window
        .set_content_protected(hide)
        .map_err(|e| e.to_string())
}

/// Toggle click-through via Win32 styles immediately (tao applies the same flag
/// asynchronously — too slow / racy for Ctrl hold-to-interact).
pub fn set_ignore_cursor_events(window: &WebviewWindow, ignore: bool) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0;
    // SAFETY: HWND from Tauri is valid while the window exists; style toggle is standard.
    unsafe {
        let mut ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if ignore {
            ex |= (WS_EX_TRANSPARENT | WS_EX_LAYERED) as i32;
        } else {
            // Drop pass-through; keep LAYERED so transparent compositing still works.
            ex &= !(WS_EX_TRANSPARENT as i32);
            ex |= WS_EX_LAYERED as i32;
        }
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex);
        // Force hit-test refresh — otherwise mouse stays "through" until it moves.
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }

    // Keep tao WindowFlags in sync (best-effort; applied on the UI thread).
    let _ = window.set_ignore_cursor_events(ignore);
    Ok(())
}
