use serde::{Deserialize, Serialize};

use crate::audio::{
    list_devices_async, validate_audio_setup, AudioDeviceInfo, AudioSetupValidation,
};
use crate::config::{AppConfig, DeviceRef, PipelineOutputMode};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicesResponse {
    pub devices: Vec<AudioDeviceInfo>,
}

#[tauri::command]

pub async fn list_audio_devices() -> Result<DevicesResponse, String> {
    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    Ok(DevicesResponse { devices })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateAudioSetupRequest {
    pub my_language: String,
    pub meeting_language: String,
    pub user_mic: DeviceRef,
    pub teams_mic_feed: DeviceRef,
    pub meeting_capture: DeviceRef,
    pub local_playback: DeviceRef,
    #[serde(default)]
    pub outbound_mode: PipelineOutputMode,
    #[serde(default)]
    pub inbound_mode: PipelineOutputMode,
}

#[tauri::command]
pub async fn validate_audio_setup_cmd(
    request: ValidateAudioSetupRequest,
) -> Result<AudioSetupValidation, String> {
    let partial = AppConfig {
        gemini_api_key: String::new(),
        my_language: request.my_language,
        meeting_language: request.meeting_language,
        user_mic: request.user_mic,
        teams_mic_feed: request.teams_mic_feed,
        meeting_capture: request.meeting_capture,
        local_playback: request.local_playback,
        outbound_mode: request.outbound_mode,
        inbound_mode: request.inbound_mode,
        ..AppConfig::default()
    };

    let devices = list_devices_async().await.map_err(|e| e.to_string())?;
    Ok(validate_audio_setup(&partial, &devices))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewInboundDuckingRequest {
    pub enabled: bool,
    pub gain: f32,
}

/// Hot-preview Meeting → You original underlay while Settings draft is dirty.
/// Does not mutate AppConfig or persist to disk — Save still required.
#[tauri::command]
pub async fn preview_inbound_ducking(
    request: PreviewInboundDuckingRequest,
    engine: tauri::State<'_, crate::runtime::engine::SharedEngine>,
) -> Result<(), String> {
    let gain = request.gain.clamp(0.0, 0.5);
    let guard = engine.lock().await;
    guard.preview_inbound_ducking(request.enabled, gain);
    Ok(())
}
