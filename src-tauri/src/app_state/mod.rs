//! Published UI/read-model state derived from engine + config.
//!
//! Owns snapshot types (`AppSnapshot`, …) and config-change sync
//! (`sync_app_state_after_config_change`). The former top-level
//! `app_snapshot` module was folded here.

mod audio_path;
mod derive;
mod snapshot;
mod sync;

pub use audio_path::AudioPathPhase;
pub use derive::{
    build_app_status, build_column_ui_snapshot, build_setup_state, devices_list_changed,
    setup_semantically_eq, snapshot_semantically_eq, EngineView,
};
pub use snapshot::{
    AppSnapshot, ColumnIdleBadgeSnapshot, ColumnUiSnapshot, ColumnUiState, ConfigState,
    DeviceCatalogState, SetupState,
};
pub use sync::sync_app_state_after_config_change;

pub use derive::build_setup_state as compute_setup_state;

#[derive(Debug, Clone)]
pub struct PublishContext {
    pub config: crate::config::AppConfig,
    pub devices: Vec<crate::audio::AudioDeviceInfo>,
    pub now_ms: u64,
}

impl PublishContext {
    pub fn new(
        config: crate::config::AppConfig,
        devices: Vec<crate::audio::AudioDeviceInfo>,
        now_ms: u64,
    ) -> Self {
        Self {
            config,
            devices,
            now_ms,
        }
    }
}
