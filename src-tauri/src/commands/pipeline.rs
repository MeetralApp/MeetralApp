use tauri::{AppHandle, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::app_state::sync_app_state_after_config_change;
use crate::audio::list_devices_async;
use crate::config::{
    AppConfig, AudioPathMode, InboundVoiceOutput, OutboundVoiceOutput, PipelineOutputMode,
};
use crate::config_store;
use crate::runtime::engine::{
    ensure_direct_audio_shared, run_finish_start_inbound, run_finish_start_outbound, AppStatus,
    SharedEngine,
};

async fn spawn_start_outbound(
    app: AppHandle,

    engine: SharedEngine,

    config: AppConfig,
) -> Result<AppStatus, String> {
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;

    {
        let mut guard = engine.lock().await;

        guard
            .begin_start_outbound(config.clone(), &devices, &app)
            .map_err(|e| e.to_string())?;
    }

    let engine_bg = engine.clone();

    let app_bg = app.clone();

    tokio::spawn(async move {
        run_finish_start_outbound(engine_bg, config, app_bg).await;
    });

    let guard = engine.lock().await;

    Ok(guard.status_snapshot())
}

async fn spawn_start_inbound(
    app: AppHandle,

    engine: SharedEngine,

    config: AppConfig,
) -> Result<AppStatus, String> {
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;

    {
        let mut guard = engine.lock().await;

        guard
            .begin_start_inbound(config.clone(), &devices, &app)
            .map_err(|e| e.to_string())?;
    }

    let engine_bg = engine.clone();

    let app_bg = app.clone();

    tokio::spawn(async move {
        run_finish_start_inbound(engine_bg, config, app_bg).await;
    });

    let guard = engine.lock().await;

    Ok(guard.status_snapshot())
}

#[tauri::command]

pub async fn start_outbound(
    app: AppHandle,

    engine: State<'_, SharedEngine>,

    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();

    spawn_start_outbound(app, engine.inner().clone(), config).await
}

#[tauri::command]

pub async fn start_inbound(
    app: AppHandle,

    engine: State<'_, SharedEngine>,

    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();

    spawn_start_inbound(app, engine.inner().clone(), config).await
}

#[tauri::command]

pub async fn stop_outbound(
    app: AppHandle,

    engine: State<'_, SharedEngine>,

    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();

    // Teardown runs outside the engine lock so bridge/TTS joins cannot stall IPC.
    crate::runtime::engine::stop_outbound_shared(engine.inner(), &config, &app).await;

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]

pub async fn stop_inbound(
    app: AppHandle,

    engine: State<'_, SharedEngine>,

    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();

    // Teardown runs outside the engine lock so bridge/TTS joins cannot stall IPC.
    crate::runtime::engine::stop_inbound_shared(engine.inner(), &config, &app).await;

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_outbound_output_mode(
    app: AppHandle,
    mode: PipelineOutputMode,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let merged = {
        let mut guard = config_store.lock().await;
        guard.outbound_mode = mode;
        guard.clone()
    };
    let merged_for_disk = merged.clone();
    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || config_store::save(&app_bg, &merged_for_disk))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    {
        let mut guard = engine.lock().await;
        guard.set_outbound_output_mode(mode, &merged, &app).await?;
    }

    sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_outbound_voice_output(
    app: AppHandle,
    voice_output: OutboundVoiceOutput,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let mut config = config_store.lock().await.clone();
    {
        let mut guard = engine.lock().await;
        crate::runtime::engine::set_outbound_voice_output(
            &mut guard,
            &mut config,
            voice_output,
            &app,
        )
        .await?;
    }

    {
        let mut guard = config_store.lock().await;
        *guard = config.clone();
    }
    let config_for_disk = config.clone();
    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || config_store::save(&app_bg, &config_for_disk))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    sync_app_state_after_config_change(&app, engine.inner(), &config).await?;

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_inbound_voice_output(
    app: AppHandle,
    voice_output: InboundVoiceOutput,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let mut config = config_store.lock().await.clone();
    {
        let mut guard = engine.lock().await;
        crate::runtime::engine::set_inbound_voice_output(
            &mut guard,
            &mut config,
            voice_output,
            &app,
        )
        .await?;
    }

    {
        let mut guard = config_store.lock().await;
        *guard = config.clone();
    }
    let config_for_disk = config.clone();
    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || config_store::save(&app_bg, &config_for_disk))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    sync_app_state_after_config_change(&app, engine.inner(), &config).await?;

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_inbound_output_mode(
    app: AppHandle,
    mode: PipelineOutputMode,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let merged = {
        let mut guard = config_store.lock().await;
        guard.inbound_mode = mode;
        guard.clone()
    };
    let merged_for_disk = merged.clone();
    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || config_store::save(&app_bg, &merged_for_disk))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    {
        let mut guard = engine.lock().await;
        guard.set_inbound_output_mode(mode, &merged, &app).await?;
    }

    sync_app_state_after_config_change(&app, engine.inner(), &merged).await?;

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_outbound_audio_mode(
    app: AppHandle,
    mode: AudioPathMode,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    let needs_finish = match mode {
        AudioPathMode::Direct => {
            // apply(Direct) with joins outside the engine lock.
            crate::runtime::engine::apply_audio_path_to_direct_shared(
                crate::runtime::engine::Direction::Outbound,
                engine.inner(),
                &config,
                &app,
            )
            .await?;
            false
        }
        AudioPathMode::Translate => {
            let mut guard = engine.lock().await;
            if !guard.is_outbound_busy() {
                guard
                    .begin_start_outbound(config.clone(), &devices, &app)
                    .map_err(|e| e.to_string())?;
                true
            } else {
                false
            }
        }
    };

    if needs_finish {
        let engine_bg = engine.inner().clone();
        let app_bg = app.clone();
        tokio::spawn(async move {
            run_finish_start_outbound(engine_bg, config, app_bg).await;
        });
    }

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn set_inbound_audio_mode(
    app: AppHandle,
    mode: AudioPathMode,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    let needs_finish = match mode {
        AudioPathMode::Direct => {
            // apply(Direct) with joins outside the engine lock.
            crate::runtime::engine::apply_audio_path_to_direct_shared(
                crate::runtime::engine::Direction::Inbound,
                engine.inner(),
                &config,
                &app,
            )
            .await?;
            false
        }
        AudioPathMode::Translate => {
            let mut guard = engine.lock().await;
            if !guard.is_inbound_busy() {
                guard
                    .begin_start_inbound(config.clone(), &devices, &app)
                    .map_err(|e| e.to_string())?;
                true
            } else {
                false
            }
        }
    };

    if needs_finish {
        let engine_bg = engine.inner().clone();
        let app_bg = app.clone();
        tokio::spawn(async move {
            run_finish_start_inbound(engine_bg, config, app_bg).await;
        });
    }

    let guard = engine.lock().await;
    Ok(guard.status_snapshot())
}

#[tauri::command]
pub async fn ensure_direct_audio(
    app: AppHandle,
    engine: State<'_, SharedEngine>,
    config_store: State<'_, AsyncMutex<AppConfig>>,
) -> Result<AppStatus, String> {
    let config = config_store.lock().await.clone();
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    let now_ms = crate::audio::monotonic_ms();
    let engine_bg = engine.inner().clone();
    ensure_direct_audio_shared(engine_bg.clone(), &config, &devices, &app).await?;
    let mut guard = engine.lock().await;
    guard.ensure_watchdog(app.clone(), engine_bg);
    guard.update_publish_context(&config, &devices, now_ms);
    guard.publish(&app);
    Ok(guard.status_snapshot())
}
