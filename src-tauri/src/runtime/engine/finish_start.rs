use super::{Direction, SharedEngine};
use crate::config::AppConfig;
use crate::pipeline::{InboundPipeline, OutboundPipeline};
use tauri::AppHandle;

pub async fn run_finish_start_outbound(engine: SharedEngine, config: AppConfig, app: AppHandle) {
    let devices = match crate::audio::list_devices_async().await {
        Ok(devices) => devices,
        Err(e) => {
            let mut guard = engine.lock().await;
            guard
                .fail_start(Direction::Outbound, e.to_string(), &config, &[], &app)
                .await;
            return;
        }
    };

    let prepare = {
        let mut guard = engine.lock().await;
        match guard.prepare_finish_start_outbound(&app, engine.clone(), &devices) {
            Ok(Some(prepare)) => prepare,
            Ok(None) => return,
            Err(e) => {
                guard
                    .fail_start(Direction::Outbound, e, &config, &devices, &app)
                    .await;
                return;
            }
        }
    };

    {
        let handle = {
            let mut guard = engine.lock().await;
            guard.take_direct_outbound_for_stop()
        };
        super::lock_scope::join_optional_capture(handle).await;
    }

    if config.needs_custom_tts_for_outbound() {
        if let Err(e) = crate::runtime::factories::validate_outbound_custom_voice(&config).await {
            prepare.cancel.cancel();
            let mut guard = engine.lock().await;
            guard
                .fail_start(Direction::Outbound, e, &config, &prepare.devices, &app)
                .await;
            return;
        }
    }

    let pcm_drops = {
        let guard = engine.lock().await;
        guard.pcm_frames_dropped.clone()
    };

    let bridge_result = OutboundPipeline::connect_for_start(
        &config,
        prepare.transcript_tx.clone(),
        prepare.fatal_tx.clone(),
        prepare.status_tx.clone(),
        prepare.cancel.clone(),
        pcm_drops,
    )
    .await;

    let mut guard = engine.lock().await;
    guard
        .complete_finish_start_outbound(prepare, config, &app, bridge_result)
        .await;
    guard.ensure_watchdog(app.clone(), engine.clone());
}

pub async fn run_finish_start_inbound(engine: SharedEngine, config: AppConfig, app: AppHandle) {
    let devices = match crate::audio::list_devices_async().await {
        Ok(devices) => devices,
        Err(e) => {
            let mut guard = engine.lock().await;
            guard
                .fail_start(Direction::Inbound, e.to_string(), &config, &[], &app)
                .await;
            return;
        }
    };

    let prepare = {
        let mut guard = engine.lock().await;
        match guard.prepare_finish_start_inbound(&app, engine.clone(), &devices) {
            Ok(Some(prepare)) => prepare,
            Ok(None) => return,
            Err(e) => {
                guard
                    .fail_start(Direction::Inbound, e, &config, &devices, &app)
                    .await;
                return;
            }
        }
    };

    let pcm_drops = {
        let guard = engine.lock().await;
        guard.pcm_frames_dropped.clone()
    };

    let bridge_result = InboundPipeline::connect_bridge(
        &config,
        prepare.transcript_tx.clone(),
        prepare.fatal_tx.clone(),
        prepare.status_tx.clone(),
        prepare.cancel.clone(),
        pcm_drops,
    )
    .await;

    let mut guard = engine.lock().await;
    guard
        .complete_finish_start_inbound(prepare, config, &app, bridge_result)
        .await;
    guard.ensure_watchdog(app.clone(), engine.clone());
}
