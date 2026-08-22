use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};
use tokio::sync::Mutex as AsyncMutex;

use crate::config::AppConfig;
use crate::runtime::engine::{AppStatus, PipelineState, SharedEngine};

const MENU_SHOW: &str = "tray_show";
const MENU_TOGGLE_OVERLAY: &str = "tray_toggle_overlay";
const MENU_TOGGLE_CLICK_THROUGH: &str = "tray_toggle_click_through";
const MENU_TOGGLE_KEEP_DIRECT: &str = "tray_toggle_keep_direct";
const MENU_OUTBOUND_STATUS: &str = "tray_outbound_status";
const MENU_INBOUND_STATUS: &str = "tray_inbound_status";
const MENU_TOGGLE_MIC: &str = "tray_toggle_mic";
const MENU_TOGGLE_SPEAKER: &str = "tray_toggle_speaker";
const MENU_QUIT: &str = "tray_quit";

pub struct RuntimeConfig {
    pub close_to_tray: Arc<AtomicBool>,
    pub shutting_down: Arc<AtomicBool>,
}

impl RuntimeConfig {
    pub fn new(close_to_tray: bool) -> Self {
        Self {
            close_to_tray: Arc::new(AtomicBool::new(close_to_tray)),
            shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_close_to_tray(&self, value: bool) {
        self.close_to_tray.store(value, Ordering::Relaxed);
    }

    pub fn begin_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }
}

/// References to tray menu items updated when engine status changes.
pub struct TrayMenuState {
    outbound_status: MenuItem<Wry>,
    inbound_status: MenuItem<Wry>,
    mic_mute: CheckMenuItem<Wry>,
    speaker_mute: CheckMenuItem<Wry>,
    overlay_visible: CheckMenuItem<Wry>,
    click_through: CheckMenuItem<Wry>,
    keep_direct_audio: CheckMenuItem<Wry>,
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let initial_keep_direct = app
        .try_state::<AsyncMutex<AppConfig>>()
        .and_then(|store| store.try_lock().ok().map(|g| g.keep_direct_audio))
        .unwrap_or(true);
    let initial_click_through = app
        .try_state::<AsyncMutex<AppConfig>>()
        .and_then(|store| store.try_lock().ok().map(|g| g.overlay.click_through))
        .unwrap_or(false);
    let initial_overlay = app
        .try_state::<AsyncMutex<AppConfig>>()
        .and_then(|store| store.try_lock().ok().map(|g| g.overlay.enabled))
        .unwrap_or(false)
        || overlay_window_visible(app);

    let show = MenuItem::with_id(app, MENU_SHOW, "Show", true, None::<&str>)?;
    let overlay = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_OVERLAY,
        "Show overlay",
        true,
        initial_overlay,
        None::<&str>,
    )?;
    let click_through = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_CLICK_THROUGH,
        "Overlay click-through",
        true,
        initial_click_through,
        None::<&str>,
    )?;
    let keep_direct = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_KEEP_DIRECT,
        "Keep Direct audio",
        true,
        initial_keep_direct,
        None::<&str>,
    )?;
    let sep_status = PredefinedMenuItem::separator(app)?;
    let outbound_status =
        MenuItem::with_id(app, MENU_OUTBOUND_STATUS, "You: Off", false, None::<&str>)?;
    let inbound_status = MenuItem::with_id(
        app,
        MENU_INBOUND_STATUS,
        "Meeting: Off",
        false,
        None::<&str>,
    )?;
    let sep_mute = PredefinedMenuItem::separator(app)?;
    let mic_mute = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_MIC,
        "Mute microphone (not active)",
        false,
        false,
        None::<&str>,
    )?;
    let speaker_mute = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_SPEAKER,
        "Mute meeting audio (not active)",
        false,
        false,
        None::<&str>,
    )?;
    let sep_quit = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &overlay,
            &click_through,
            &keep_direct,
            &sep_status,
            &outbound_status,
            &inbound_status,
            &sep_mute,
            &mic_mute,
            &speaker_mute,
            &sep_quit,
            &quit,
        ],
    )?;

    app.manage(TrayMenuState {
        outbound_status: outbound_status.clone(),
        inbound_status: inbound_status.clone(),
        mic_mute: mic_mute.clone(),
        speaker_mute: speaker_mute.clone(),
        overlay_visible: overlay.clone(),
        click_through: click_through.clone(),
        keep_direct_audio: keep_direct.clone(),
    });

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("missing default window icon")))?;

    let app_handle = app.clone();
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .tooltip("Meetral")
        .on_menu_event(move |app, event| {
            handle_menu_event(app, &app_handle, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_main_window(app);
            }
        })
        .build(app)?;

    let app_init = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(engine) = app_init.try_state::<SharedEngine>() {
            let guard = engine.lock().await;
            update_tray_ui(&app_init, &guard.status_snapshot());
        }
    });

    Ok(())
}

fn handle_menu_event(app: &AppHandle, app_handle: &AppHandle, id: &str) {
    match id {
        MENU_SHOW => show_main_window(app),
        MENU_TOGGLE_OVERLAY => {
            if let Err(error) = crate::overlay::toggle_enabled(app) {
                tracing::warn!("tray overlay toggle failed: {error}");
            }
        }
        MENU_TOGGLE_CLICK_THROUGH => match crate::overlay::toggle_click_through(app) {
            Ok(on) => {
                if let Some(menu) = app.try_state::<TrayMenuState>() {
                    let _ = menu.click_through.set_checked(on);
                }
            }
            Err(error) => {
                tracing::warn!("tray overlay click-through toggle failed: {error}");
            }
        },
        MENU_TOGGLE_KEEP_DIRECT => {
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = toggle_keep_direct_audio(&app).await {
                    tracing::warn!("tray keep-direct-audio toggle failed: {error}");
                }
            });
        }
        MENU_TOGGLE_MIC => {
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                toggle_mic_from_tray(&app).await;
            });
        }
        MENU_TOGGLE_SPEAKER => {
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                toggle_speaker_from_tray(&app).await;
            });
        }
        MENU_QUIT => {
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                graceful_quit(&app).await;
            });
        }
        _ => {}
    }
}

async fn toggle_keep_direct_audio(app: &AppHandle) -> Result<(), String> {
    let store = app
        .try_state::<AsyncMutex<AppConfig>>()
        .ok_or_else(|| "config store missing".to_string())?;
    let engine = app
        .try_state::<SharedEngine>()
        .ok_or_else(|| "engine missing".to_string())?;

    let merged = {
        let mut guard = store.lock().await;
        guard.keep_direct_audio = !guard.keep_direct_audio;
        guard.clone()
    };

    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.keep_direct_audio.set_checked(merged.keep_direct_audio);
    }

    let save_cfg = merged.clone();
    let app_save = app.clone();
    tokio::task::spawn_blocking(move || crate::config_store::save(&app_save, &save_cfg))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    crate::app_state::sync_app_state_after_config_change(app, engine.inner(), &merged).await?;

    if merged.keep_direct_audio {
        let devices = crate::audio::list_devices_async()
            .await
            .map_err(|e| e.to_string())?;
        let engine_bg = engine.inner().clone();
        let _ = crate::runtime::engine::ensure_direct_audio_shared(
            engine_bg.clone(),
            &merged,
            &devices,
            app,
        )
        .await;
        let mut guard = engine.lock().await;
        guard.ensure_watchdog(app.clone(), engine_bg);
        guard.publish(app);
    }

    Ok(())
}

async fn toggle_mic_from_tray(app: &AppHandle) {
    let Some(engine) = app.try_state::<SharedEngine>() else {
        return;
    };
    let mut guard = engine.lock().await;
    if !outbound_audio_active(&guard.status_snapshot()) {
        update_tray_ui(app, &guard.status_snapshot());
        return;
    }
    let next = !guard.mic_muted();
    if let Err(e) = guard.set_mic_muted(next, app) {
        tracing::warn!("tray mic mute failed: {e}");
        update_tray_ui(app, &guard.status_snapshot());
    }
}

async fn toggle_speaker_from_tray(app: &AppHandle) {
    let Some(engine) = app.try_state::<SharedEngine>() else {
        return;
    };
    let mut guard = engine.lock().await;
    if !inbound_audio_active(&guard.status_snapshot()) {
        update_tray_ui(app, &guard.status_snapshot());
        return;
    }
    let next = !guard.speaker_muted();
    if let Err(e) = guard.set_speaker_muted(next, app) {
        tracing::warn!("tray speaker mute failed: {e}");
        update_tray_ui(app, &guard.status_snapshot());
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let (Some(config_store), Some(engine)) = (
            app.try_state::<AsyncMutex<AppConfig>>(),
            app.try_state::<SharedEngine>(),
        ) {
            let config = config_store.lock().await.clone();
            let Ok(devices) = crate::audio::list_devices_async().await else {
                return;
            };
            let now_ms = crate::audio::monotonic_ms();
            let engine_bg = engine.inner().clone();
            let _ = crate::runtime::engine::ensure_direct_audio_shared(
                engine_bg.clone(),
                &config,
                &devices,
                &app,
            )
            .await;
            let mut guard = engine.lock().await;
            guard.ensure_watchdog(app.clone(), engine_bg);
            guard.update_publish_context(&config, &devices, now_ms);
            guard.publish(&app);
        }
    });
}

pub async fn graceful_quit(app: &AppHandle) {
    if let Some(runtime) = app.try_state::<RuntimeConfig>() {
        runtime.begin_shutdown();
    }

    if let Some(engine) = app.try_state::<SharedEngine>() {
        let mut guard = engine.lock().await;
        guard.graceful_shutdown(app).await;
    }

    // Do not call tray.set_visible(false) before exit — on Windows, dropping a hidden
    // tray icon prints "Error removing system tray icon" (tray-icon#289).
    // Close webviews before exit so Chromium can tear down HWNDs cleanly (reduces
    // "Failed to unregister class Chrome_WidgetWin_0" noise from WebView2).
    if let Some(window) = app.get_webview_window(crate::overlay::OVERLAY_LABEL) {
        let _ = window.destroy();
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.destroy();
    }

    // Allow WebView2 to destroy HWND before process exit.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Flush buffered logs before the process exits (app.exit -> process::exit does
    // not run destructors, so the log worker must be flushed explicitly here).
    if let Some(log_guard) = app.try_state::<crate::logging::LogGuard>() {
        log_guard.flush_and_close();
    }

    app.exit(0);
}

pub fn update_tray_ui(app: &AppHandle, status: &AppStatus) {
    if let Some(tray) = app.tray_by_id("main") {
        let outbound = pipeline_label(&status.outbound);
        let inbound = pipeline_label(&status.inbound);
        let tooltip = format!("You: {outbound} | Meeting: {inbound}");
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    let Some(menu) = app.try_state::<TrayMenuState>() else {
        return;
    };

    let outbound_active = outbound_audio_active(status);
    let inbound_active = inbound_audio_active(status);

    let _ = menu.outbound_status.set_text(format!(
        "You: {outbound}",
        outbound = pipeline_label(&status.outbound)
    ));
    let _ = menu.inbound_status.set_text(format!(
        "Meeting: {inbound}",
        inbound = pipeline_label(&status.inbound)
    ));

    let mic_label = if outbound_active {
        "Mute microphone"
    } else {
        "Mute microphone (not active)"
    };
    let speaker_label = if inbound_active {
        "Mute meeting audio"
    } else {
        "Mute meeting audio (not active)"
    };

    let _ = menu.mic_mute.set_text(mic_label);
    let _ = menu.mic_mute.set_enabled(outbound_active);
    let _ = menu.mic_mute.set_checked(status.mic_muted);

    let _ = menu.speaker_mute.set_text(speaker_label);
    let _ = menu.speaker_mute.set_enabled(inbound_active);
    let _ = menu.speaker_mute.set_checked(status.speaker_muted);

    let (click_through, keep_direct, overlay_enabled) = app
        .try_state::<AsyncMutex<AppConfig>>()
        .and_then(|store| {
            store.try_lock().ok().map(|g| {
                (
                    g.overlay.click_through,
                    g.keep_direct_audio,
                    g.overlay.enabled,
                )
            })
        })
        .unwrap_or((false, true, false));
    let overlay_on = overlay_enabled || overlay_window_visible(app);
    let _ = menu.overlay_visible.set_checked(overlay_on);
    let _ = menu.click_through.set_checked(click_through);
    let _ = menu.keep_direct_audio.set_checked(keep_direct);
}

fn overlay_window_visible(app: &AppHandle) -> bool {
    app.get_webview_window(crate::overlay::OVERLAY_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

fn outbound_audio_active(status: &AppStatus) -> bool {
    matches!(
        status.outbound,
        PipelineState::Direct
            | PipelineState::Starting
            | PipelineState::Stopping
            | PipelineState::Active
    )
}

fn inbound_audio_active(status: &AppStatus) -> bool {
    matches!(
        status.inbound,
        PipelineState::Direct
            | PipelineState::Starting
            | PipelineState::Stopping
            | PipelineState::Active
    )
}

fn pipeline_label(state: &PipelineState) -> &'static str {
    match state {
        PipelineState::Off => "Off",
        PipelineState::Direct => "Direct",
        PipelineState::Starting => "Starting",
        PipelineState::Stopping => "Stopping",
        PipelineState::Active => "Translate",
        PipelineState::Error => "Error",
    }
}

/// Sync tray "Show overlay" checkbox with persisted enabled flag.
pub fn set_overlay_menu_checked(app: &AppHandle, checked: bool) {
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.overlay_visible.set_checked(checked);
    }
}
