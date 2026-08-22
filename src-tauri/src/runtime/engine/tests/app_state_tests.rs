use super::super::TranslationEngine;
use crate::app_state::{build_setup_state, EngineView};
use crate::audio::CaptureHeartbeat;
use crate::runtime::engine::{
    BridgeConnectionState, PipelineState, AUDIO_HEARTBEAT_STALE_MS, AUDIO_STARTUP_GRACE_MS,
};
use crate::test_support::{sample_config, sample_devices};

#[test]
fn lost_audio_clears_when_devices_ready_again() {
    let config = sample_config();
    let devices = sample_devices();
    let mut engine = TranslationEngine::new();
    engine
        .outbound_side
        .audio_path
        .mark_lost(crate::runtime::engine::unix_ms_now_u64());

    engine.try_recover_lost_audio_paths(&config, &devices);

    assert!(engine.outbound_side.audio_path.is_ok());
}

#[test]
fn lost_audio_stays_lost_when_devices_invalid() {
    let config = sample_config();
    let mut engine = TranslationEngine::new();
    engine.outbound_side.audio_path.mark_lost(1);
    engine.try_recover_lost_audio_paths(&config, &[]);
    assert!(engine.outbound_side.audio_path.is_lost());
}

#[test]
fn lost_audio_stays_lost_when_outbound_busy() {
    let config = sample_config();
    let devices = sample_devices();
    let mut engine = TranslationEngine::new();
    engine.outbound_side.audio_path.mark_lost(1);
    engine.outbound_side.starting = true;
    engine.try_recover_lost_audio_paths(&config, &devices);
    assert!(engine.outbound_side.audio_path.is_lost());
}

#[test]
fn capture_heartbeat_confirmed_matrix() {
    let hb = CaptureHeartbeat::new();
    assert!(!TranslationEngine::capture_heartbeat_confirmed(
        &hb, None, 10_000
    ));

    hb.touch();
    let now = crate::audio::monotonic_ms().max(1);
    assert!(TranslationEngine::capture_heartbeat_confirmed(
        &hb,
        Some(now),
        now,
    ));

    // Force last frame far in the past relative to now (bypass startup grace).
    let stale_now = hb.last_frame_ms() + AUDIO_HEARTBEAT_STALE_MS + AUDIO_STARTUP_GRACE_MS + 1;
    assert!(!TranslationEngine::capture_heartbeat_confirmed(
        &hb,
        Some(0),
        stale_now,
    ));
}

#[test]
fn cleared_lost_enables_translate_in_derived_setup() {
    let config = sample_config();
    let devices = sample_devices();
    let mut engine = TranslationEngine::new();
    engine.update_publish_context(&config, &devices, 1);
    engine.outbound_side.audio_path.mark_lost(1);
    engine.try_recover_lost_audio_paths(&config, &devices);

    let view = EngineView {
        config: &config,
        devices: &devices,
        outbound_pipeline: PipelineState::Off,
        inbound_pipeline: PipelineState::Off,
        outbound_audio_raw: engine.outbound_side.audio_path.connection_state(),
        inbound_audio_raw: engine.inbound_side.audio_path.connection_state(),
        outbound_audio_attempt: None,
        inbound_audio_attempt: None,
        runtime_error: None,
        outbound_active_since: None,
        inbound_active_since: None,
        outbound_bridge: BridgeConnectionState::Idle,
        inbound_bridge: BridgeConnectionState::Idle,
        outbound_bridge_attempt: None,
        inbound_bridge_attempt: None,
        running: false,
        mic_muted: false,
        speaker_muted: false,
    };
    let setup = build_setup_state(&view, 1);
    assert!(setup.can_translate_outbound);
    assert_eq!(setup.outbound_idle_badge.kind, "ready");
}
