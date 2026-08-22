use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEvent {
    pub direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// True while the provider turn is still in progress (only cleared on commit).
    pub interim: bool,
    pub turn_complete: bool,
    #[serde(default)]
    pub input_segment_finished: bool,
    #[serde(default)]
    pub output_segment_finished: bool,
    #[serde(default)]
    pub connection_gap: bool,
    /// Whether text is a full live-utterance snapshot rather than a delta.
    #[serde(default)]
    pub replace_live: bool,
    /// Populated by transcript relay from SegmentEngine (not from the provider).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_translated: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BridgeStatusEvent {
    Reconnecting {
        direction: String,
        attempt: u32,
    },
    Ready {
        direction: String,
        reconnected: bool,
    },
}
