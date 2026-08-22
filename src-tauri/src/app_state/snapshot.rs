use serde::Serialize;

use crate::audio::{AudioDeviceInfo, AudioSetupValidation};
use crate::config::ConfigView;
use crate::runtime::engine::{AppStatus, AudioConnectionState, PipelineState};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ColumnIdleBadgeSnapshot {
    pub kind: String,
    pub label: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ColumnUiState {
    pub pipeline: PipelineState,
    pub audio_connection: AudioConnectionState,
    pub can_direct: bool,
    pub can_translate: bool,
    pub translate_disabled_reason: Option<String>,
    pub idle_badge: ColumnIdleBadgeSnapshot,
    pub pipeline_live: bool,
    pub mute_enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ColumnUiSnapshot {
    pub outbound: ColumnUiState,
    pub inbound: ColumnUiState,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCatalogState {
    pub revision: u64,
    pub devices: Vec<AudioDeviceInfo>,
    pub enumerated_at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub revision: u64,
    pub api_key_configured: bool,
    pub can_direct_outbound: bool,
    pub can_direct_inbound: bool,
    pub can_translate_outbound: bool,
    pub can_translate_inbound: bool,
    pub outbound_idle_badge: ColumnIdleBadgeSnapshot,
    pub inbound_idle_badge: ColumnIdleBadgeSnapshot,
    pub audio: AudioSetupValidation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigState {
    pub revision: u64,
    #[serde(flatten)]
    pub config: ConfigView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub revision: u64,
    pub runtime: AppStatus,
    pub setup: SetupState,
    pub columns: ColumnUiSnapshot,
    pub devices: DeviceCatalogState,
    pub config: ConfigState,
}
