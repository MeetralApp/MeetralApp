use tauri::{AppHandle, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::config::{AppConfig, OverlaySettings};
use crate::overlay;
use crate::runtime::engine::SharedEngine;

#[tauri::command]
pub fn overlay_show(app: AppHandle) -> Result<(), String> {
    overlay::show(&app)
}

#[tauri::command]
pub async fn overlay_hide(
    app: AppHandle,
    store: State<'_, AsyncMutex<AppConfig>>,
    engine: State<'_, SharedEngine>,
) -> Result<(), String> {
    // Close = Show transcript overlay OFF (same as Settings / Tray uncheck).
    let merged = {
        let mut guard = store.lock().await;
        guard.overlay.enabled = false;
        guard.clone()
    };

    crate::overlay::apply_settings(&app, &merged.overlay)?;
    crate::tray::set_overlay_menu_checked(&app, false);

    let save_cfg = merged.clone();
    let app_save = app.clone();
    tokio::task::spawn_blocking(move || crate::config_store::save(&app_save, &save_cfg))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    crate::app_state::sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;
    Ok(())
}

#[tauri::command]
pub fn overlay_toggle(app: AppHandle) -> Result<bool, String> {
    overlay::toggle_enabled(&app)
}

#[tauri::command]
pub fn overlay_preview(app: AppHandle) -> Result<(), String> {
    overlay::preview(&app)
}

#[tauri::command]
pub fn overlay_is_preview_mode() -> bool {
    overlay::is_preview_mode()
}

#[tauri::command]
pub fn overlay_set_pointer_interactive(app: AppHandle, interactive: bool) -> Result<(), String> {
    overlay::set_pointer_interactive(&app, interactive);
    Ok(())
}

#[tauri::command]
pub fn overlay_set_offset(app: AppHandle, offset_x: f64, offset_y: f64) -> Result<(), String> {
    overlay::set_offset(&app, offset_x, offset_y)
}

#[tauri::command]
pub fn overlay_set_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    overlay::set_size(&app, width, height)
}

/// "Xem trong app" from the overlay — show + focus main window.
#[tauri::command]
pub fn focus_main_window(app: AppHandle) {
    crate::tray::show_main_window(&app);
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySettingsPatch {
    pub opacity: Option<f32>,
    pub click_through: Option<bool>,
}

#[tauri::command]
pub async fn overlay_patch_settings(
    app: AppHandle,
    patch: OverlaySettingsPatch,
    store: State<'_, AsyncMutex<AppConfig>>,
    engine: State<'_, SharedEngine>,
) -> Result<OverlaySettings, String> {
    let (overlay, merged) = {
        let mut guard = store.lock().await;
        if let Some(opacity) = patch.opacity {
            guard.overlay.opacity = opacity;
        }
        if let Some(click_through) = patch.click_through {
            guard.overlay.click_through = click_through;
        }
        guard.overlay.normalize();
        let overlay = guard.overlay.clone();
        let merged = guard.clone();
        (overlay, merged)
    };

    let app_bg = app.clone();
    let for_disk = merged.clone();
    tokio::task::spawn_blocking(move || crate::config_store::save(&app_bg, &for_disk))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    crate::app_state::sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;

    if let Err(e) = overlay::apply_settings(&app, &overlay) {
        tracing::warn!("overlay apply after patch_settings failed: {e}");
    }

    Ok(overlay)
}
