use tauri::{AppHandle, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::app_state::{AppSnapshot, DeviceCatalogState};
use crate::audio::list_devices_async;
use crate::config::AppConfig;
use crate::runtime::engine::{
    ensure_direct_audio_shared, AppStatus, DiagnosticsSnapshot, SharedEngine,
};

#[tauri::command]
pub async fn get_status(engine: State<'_, SharedEngine>) -> Result<AppStatus, String> {
    let engine = engine.lock().await;
    Ok(engine.status_snapshot())
}

#[tauri::command]
pub async fn get_app_snapshot(
    app: AppHandle,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppSnapshot, String> {
    let config = config_store.lock().await.clone();
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    let now_ms = crate::audio::monotonic_ms();
    let engine_bg = engine.inner().clone();
    {
        let mut guard = engine.lock().await;
        guard.ensure_watchdog(app.clone(), engine_bg.clone());
    }
    ensure_direct_audio_shared(engine_bg, &config, &devices, &app).await?;
    let snapshot = {
        let mut guard = engine.lock().await;
        guard.update_publish_context(&config, &devices, now_ms);
        guard.publish(&app);
        guard.build_app_snapshot()
    };
    Ok(snapshot)
}

#[tauri::command]
pub async fn refresh_device_catalog(
    app: AppHandle,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<DeviceCatalogState, String> {
    let config = config_store.lock().await.clone();
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    let now_ms = crate::audio::monotonic_ms();
    let mut guard = engine.lock().await;
    guard.update_publish_context(&config, &devices, now_ms);
    guard.publish(&app);
    guard
        .cached_device_catalog
        .clone()
        .ok_or_else(|| "device catalog unavailable".to_string())
}

#[tauri::command]
pub async fn graceful_shutdown(app: AppHandle) -> Result<(), String> {
    crate::tray::graceful_quit(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn get_diagnostics(
    engine: State<'_, SharedEngine>,
) -> Result<DiagnosticsSnapshot, String> {
    let guard = engine.lock().await;
    Ok(guard.diagnostics_snapshot())
}

#[tauri::command]
pub async fn set_mic_muted(
    muted: bool,
    engine: State<'_, SharedEngine>,
    app: AppHandle,
) -> Result<AppStatus, String> {
    let mut guard = engine.lock().await;
    guard.set_mic_muted(muted, &app)?;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_speaker_muted(
    muted: bool,
    engine: State<'_, SharedEngine>,
    app: AppHandle,
) -> Result<AppStatus, String> {
    let mut guard = engine.lock().await;
    guard.set_speaker_muted(muted, &app)?;
    Ok(guard.status_snapshot())
}
