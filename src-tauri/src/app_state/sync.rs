//! Refresh published app state after config mutations.

use tauri::AppHandle;

use crate::config::AppConfig;
use crate::runtime::engine::SharedEngine;

pub async fn sync_app_state_after_config_change(
    app: &AppHandle,
    engine: &SharedEngine,
    config: &AppConfig,
) -> Result<(), String> {
    let devices = crate::audio::list_devices_async()
        .await
        .map_err(|e| e.to_string())?;
    let now_ms = crate::audio::monotonic_ms();
    // Take standby relays under the short lock; join their threads after the
    // guard is dropped (lock_scope pattern).
    let relay_handles = {
        let mut guard = engine.lock().await;
        guard.update_publish_context(config, &devices, now_ms);
        guard.bump_config_revision(config);
        let handles = if !config.keep_direct_audio {
            guard.take_standby_direct_audio_for_stop(app)
        } else {
            Vec::new()
        };
        guard.apply_inbound_ducking_from_config(config);
        guard.publish(app);
        handles
    };
    for handle in relay_handles {
        crate::runtime::direct_relay::join_capture_handle(handle).await;
    }
    Ok(())
}
