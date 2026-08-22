use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::config::{
    default_overlay_height, default_overlay_width, OverlayPosition, OverlaySettings,
};

pub const OVERLAY_LABEL: &str = "overlay";

pub fn ensure_overlay_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(win) = app.get_webview_window(OVERLAY_LABEL) {
        return Ok(win);
    }

    let url = WebviewUrl::App("index.html#/overlay".into());
    let builder = WebviewWindowBuilder::new(app, OVERLAY_LABEL, url)
        .title("Transcript overlay")
        .inner_size(default_overlay_width(), default_overlay_height())
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .visible(false)
        .focused(false)
        .shadow(false);

    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay);

    let win = builder.build().map_err(|e| e.to_string())?;
    super::platform::harden_overlay_window(&win);
    Ok(win)
}

pub fn preset_anchor(app: &AppHandle, settings: &OverlaySettings) -> (f64, f64) {
    let (mon_x, mon_y, mon_w, mon_h) = monitor_work_area(app);
    let w = settings.width;
    let h = settings.height;
    let margin = 24.0;
    let (x, y) = match settings.position {
        OverlayPosition::BottomCenter => (mon_x + (mon_w - w) / 2.0, mon_y + mon_h - h - margin),
        OverlayPosition::BottomLeft => (mon_x + margin, mon_y + mon_h - h - margin),
        OverlayPosition::BottomRight => (mon_x + mon_w - w - margin, mon_y + mon_h - h - margin),
        OverlayPosition::TopRight => (mon_x + mon_w - w - margin, mon_y + margin),
    };
    (x, y)
}

pub fn apply_geometry(
    window: &WebviewWindow,
    app: &AppHandle,
    settings: &OverlaySettings,
) -> Result<(), String> {
    let (ax, ay) = preset_anchor(app, settings);
    let x = ax + settings.offset_x;
    let y = ay + settings.offset_y;
    let scale = window.scale_factor().unwrap_or(1.0);
    window
        .set_position(tauri::Position::Physical(PhysicalPosition::new(
            (x * scale) as i32,
            (y * scale) as i32,
        )))
        .map_err(|e| e.to_string())?;
    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        settings.width,
        settings.height,
    )));
    Ok(())
}

fn monitor_work_area(app: &AppHandle) -> (f64, f64, f64, f64) {
    // Prefer monitor containing the main window; fall back to primary.
    if let Some(main) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = main.current_monitor() {
            let pos = monitor.position();
            let size = monitor.size();
            let scale = monitor.scale_factor();
            return (
                pos.x as f64 / scale,
                pos.y as f64 / scale,
                size.width as f64 / scale,
                size.height as f64 / scale,
            );
        }
    }
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let pos = monitor.position();
        let size = monitor.size();
        let scale = monitor.scale_factor();
        return (
            pos.x as f64 / scale,
            pos.y as f64 / scale,
            size.width as f64 / scale,
            size.height as f64 / scale,
        );
    }
    (0.0, 0.0, 1920.0, 1080.0)
}
