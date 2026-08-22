use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AudioConnectionState {
    Ok,
    Reconnecting,
    Lost,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PipelineState {
    #[serde(alias = "idle")]
    Off,
    Direct,
    Starting,
    /// Graceful teardown (flush live transcript / drain bridge).
    Stopping,
    Active,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BridgeConnectionState {
    Idle,
    Ready,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub running: bool,
    pub outbound: PipelineState,
    pub inbound: PipelineState,
    pub error: Option<String>,
    pub outbound_active_since: Option<i64>,
    pub inbound_active_since: Option<i64>,
    pub outbound_bridge: BridgeConnectionState,
    pub inbound_bridge: BridgeConnectionState,
    pub outbound_reconnect_attempt: Option<u32>,
    pub inbound_reconnect_attempt: Option<u32>,
    pub outbound_audio: AudioConnectionState,
    pub inbound_audio: AudioConnectionState,
    pub outbound_audio_reconnect_attempt: Option<u32>,
    pub inbound_audio_reconnect_attempt: Option<u32>,
    pub mic_muted: bool,
    pub speaker_muted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub outbound_state: String,
    pub inbound_state: String,
    pub outbound_capture_last_ms: Option<u64>,
    pub inbound_capture_last_ms: Option<u64>,
    pub frames_dropped: u64,
    pub db_write_queue_depth: usize,
    pub reconnect_count: u32,
    pub panics_since_start: u64,
    /// Bounded control-channel drops (tts-cmd / status / fault / fanout) since
    /// process start — the aggregate behind the per-channel warn logs.
    pub control_channel_drops: u64,
}
