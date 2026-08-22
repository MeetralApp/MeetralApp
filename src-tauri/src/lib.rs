// Crate-scoped allows for structural/noisy lints that need a dedicated refactor.
// Still ~18 too_many_arguments sites (8-14 args, engine/factory wiring) —
// narrowing deferred to a params-object pass.
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
pub mod ai;
pub mod app_state;
pub mod audio;
pub mod capabilities;
pub mod commands;
pub mod config;
pub mod config_store;
pub mod debug_mode;
pub mod error;
pub mod logging;
pub mod meeting;
pub mod overlay;
pub mod pipeline;
pub mod providers;
pub mod runtime;
pub mod secret;
pub mod tray;
pub mod voice;

#[cfg(windows)]
mod windows_identity;

#[cfg(test)]
pub mod test_support;

use std::sync::Arc;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio::sync::Mutex as AsyncMutex;

use crate::config::AppConfig;
use crate::meeting::{
    new_active_meeting_id, new_meeting_idle_clock, new_shared_relay_meeting_context,
    SharedSegmentEngine, SharedTranscriptWriter,
};
use crate::runtime::engine::new_shared_engine;
use crate::tray::{setup_tray, RuntimeConfig};

use std::sync::atomic::Ordering;

fn prevent_default_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    #[cfg(debug_assertions)]
    {
        tauri_plugin_prevent_default::debug()
    }
    #[cfg(not(debug_assertions))]
    {
        tauri_plugin_prevent_default::init()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::load_dev_env_files();
    error::install_panic_hook();

    // MSI installs historically skipped NSIS sparse-identity hooks. Register
    // (and relaunch once) before the WebView starts so Task Manager groups
    // msedgewebview2 under Meetral instead of "WebView2 Manager".
    #[cfg(windows)]
    if windows_identity::ensure_before_start() {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(prevent_default_plugin())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(new_shared_engine())
        .invoke_handler(tauri::generate_handler![
            commands::list_audio_devices,
            commands::validate_audio_setup_cmd,
            commands::preview_inbound_ducking,
            commands::get_config,
            commands::save_config,
            commands::list_custom_llm_profiles,
            commands::upsert_custom_llm_profile,
            commands::delete_custom_llm_profile,
            commands::test_custom_llm_profile,
            commands::get_status,
            commands::get_app_snapshot,
            commands::refresh_device_catalog,
            commands::get_platform,
            commands::get_diagnostics,
            commands::start_outbound,
            commands::stop_outbound,
            commands::start_inbound,
            commands::stop_inbound,
            commands::set_outbound_output_mode,
            commands::set_outbound_voice_output,
            commands::set_inbound_output_mode,
            commands::set_inbound_voice_output,
            commands::set_outbound_audio_mode,
            commands::set_inbound_audio_mode,
            commands::ensure_direct_audio,
            commands::graceful_shutdown,
            commands::set_mic_muted,
            commands::set_speaker_muted,
            commands::test_api_key,
            commands::test_elevenlabs_api_key,
            commands::list_elevenlabs_voices,
            commands::list_elevenlabs_models,
            commands::list_soniox_voices,
            commands::list_soniox_tts_models,
            commands::list_soniox_stt_models,
            commands::preview_soniox_voice,
            commands::validate_elevenlabs_voice,
            commands::preview_elevenlabs_voice,
            commands::list_live_models,
            commands::seed_live_model_catalog,
            commands::get_ai_catalog,
            commands::migrate_config_for_provider,
            commands::list_meeting_folders,
            commands::create_meeting_folder,
            commands::rename_meeting_folder,
            commands::delete_meeting_folder,
            commands::reorder_meeting_folders,
            commands::create_meeting,
            commands::end_meeting,
            commands::rename_meeting,
            commands::move_meeting,
            commands::delete_meeting,
            commands::list_meetings,
            commands::get_meeting,
            commands::get_active_meeting,
            commands::list_meeting_segments,
            commands::search_segments,
            commands::search_meetings,
            commands::get_segment_neighbors,
            commands::get_meeting_summary,
            commands::list_summary_templates_cmd,
            commands::list_summary_languages_cmd,
            commands::generate_meeting_summary,
            commands::get_summary_generation_status,
            commands::list_meeting_artifacts,
            commands::set_artifact_status,
            commands::update_meeting_artifact,
            commands::create_meeting_artifact,
            commands::delete_meeting_artifact,
            commands::list_meeting_entities,
            commands::update_meeting_entity,
            commands::create_meeting_entity,
            commands::delete_meeting_entity,
            commands::update_meeting_summary,
            commands::default_meeting_audio_folder,
            commands::pick_meeting_audio_folder,
            commands::open_meeting_audio_folder,
            commands::list_meeting_audio_chunks,
            commands::meeting_audio_has_any,
            commands::verify_meeting_audio,
            commands::meeting_audio_disk_usage,
            commands::decode_meeting_audio_window,
            commands::overlay_show,
            commands::overlay_hide,
            commands::overlay_toggle,
            commands::overlay_preview,
            commands::overlay_is_preview_mode,
            commands::overlay_set_pointer_interactive,
            commands::overlay_set_offset,
            commands::overlay_set_size,
            commands::overlay_patch_settings,
            commands::focus_main_window,
        ])
        .on_page_load(|webview, payload| {
            if webview.label() != "main" {
                return;
            }
            if !matches!(payload.event(), PageLoadEvent::Finished) {
                return;
            }
            let window = webview.window();
            if let Err(e) = window.show() {
                tracing::warn!("failed to show main window after page load: {e}");
            }
            let _ = window.set_focus();
        })
        .setup(|app| {
            if let Some(log_guard) = logging::init(app.handle()) {
                app.manage(log_guard);
            }

            if crate::debug_mode::enabled() {
                tracing::info!(
                    "DEBUG_MODE enabled — STT/TTS debug traces active (soniox|elevenlabs)"
                );
            }

            let config = config_store::load_merged(app.handle()).unwrap_or_else(|e| {
                tracing::warn!("using default config: {e:#}");
                AppConfig::default()
            });

            app.manage(AsyncMutex::new(config.clone()));
            app.manage(RuntimeConfig::new(config.close_to_tray));

            let meeting_store =
                Arc::new(commands::init_meeting_store(app.handle()).map_err(|e| {
                    tracing::warn!("meeting store init failed: {e}");
                    e
                })?);
            app.manage(meeting_store.clone());
            app.manage(meeting::SummaryGenerationRegistry::new());
            app.manage(new_active_meeting_id());
            app.manage(new_meeting_idle_clock());
            let transcript_writer = Arc::new(SharedTranscriptWriter::new(0));
            app.manage(transcript_writer);
            app.manage(Arc::new(SharedSegmentEngine::new(0)));
            app.manage(new_shared_relay_meeting_context());

            setup_tray(app.handle())?;
            crate::overlay::apply_settings(app.handle(), &config.overlay)
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;

            #[cfg(target_os = "macos")]
            let overlay_shortcut = "CommandOrControl+Shift+O";
            #[cfg(not(target_os = "macos"))]
            let overlay_shortcut = "Ctrl+Shift+O";
            app.global_shortcut()
                .on_shortcut(overlay_shortcut, |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Err(error) = crate::overlay::toggle_enabled(app) {
                            tracing::warn!("overlay shortcut failed: {error}");
                        }
                    }
                })
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;

            #[cfg(target_os = "macos")]
            let click_through_shortcut = "CommandOrControl+Shift+T";
            #[cfg(not(target_os = "macos"))]
            let click_through_shortcut = "Ctrl+Shift+T";
            app.global_shortcut()
                .on_shortcut(click_through_shortcut, |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Err(error) = crate::overlay::toggle_click_through(app) {
                            tracing::warn!("overlay click-through toggle failed: {error}");
                        }
                    }
                })
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == crate::overlay::OVERLAY_LABEL {
                    api.prevent_close();
                    let _ = window.hide();
                    return;
                }
                let app = window.app_handle();
                if app
                    .try_state::<RuntimeConfig>()
                    .map(|cfg| cfg.is_shutting_down())
                    .unwrap_or(false)
                {
                    return;
                }
                let close_to_tray = app
                    .try_state::<RuntimeConfig>()
                    .map(|cfg| cfg.close_to_tray.load(Ordering::Relaxed))
                    .unwrap_or(true);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    api.prevent_close();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        crate::tray::graceful_quit(&app).await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
