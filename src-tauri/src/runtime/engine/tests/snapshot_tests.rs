use super::super::{AudioConnectionState, BridgeConnectionState, PipelineState, TranslationEngine};
use crate::ai::BridgeStatusEvent;
use crate::app_state::AudioPathPhase;

#[test]
fn new_engine_has_no_active_since() {
    let engine = TranslationEngine::new();
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_active_since, None);
    assert_eq!(snapshot.inbound_active_since, None);
}

#[test]
fn snapshot_clears_outbound_active_since_when_not_active() {
    let mut engine = TranslationEngine::new();
    engine.outbound_side.active_since = Some(1_700_000_000_000);
    engine.status.0 = PipelineState::Off;
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_active_since, None);
}

#[test]
fn snapshot_clears_inbound_active_since_when_not_active() {
    let mut engine = TranslationEngine::new();
    engine.inbound_side.active_since = Some(1_700_000_000_000);
    engine.status.1 = PipelineState::Error;
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.inbound_active_since, None);
}

#[test]
fn snapshot_exposes_active_since_when_active() {
    let mut engine = TranslationEngine::new();
    let outbound_ts = 1_700_000_000_000;
    let inbound_ts = 1_700_000_100_000;
    engine.outbound_side.active_since = Some(outbound_ts);
    engine.inbound_side.active_since = Some(inbound_ts);
    engine.status.0 = PipelineState::Active;
    engine.status.1 = PipelineState::Active;
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_active_since, Some(outbound_ts));
    assert_eq!(snapshot.inbound_active_since, Some(inbound_ts));
}

#[test]
fn hot_switch_does_not_clear_active_since_fields() {
    let mut engine = TranslationEngine::new();
    let ts = 1_700_000_000_000;
    engine.status.0 = PipelineState::Active;
    engine.outbound_side.active_since = Some(ts);
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_active_since, Some(ts));
}

#[test]
fn snapshot_clears_bridge_state_when_not_active() {
    let mut engine = TranslationEngine::new();
    engine.outbound_side.bridge = BridgeConnectionState::Reconnecting;
    engine.outbound_side.reconnect_attempt = Some(2);
    engine.status.0 = PipelineState::Off;
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_bridge, BridgeConnectionState::Idle);
    assert_eq!(snapshot.outbound_reconnect_attempt, None);
}

#[test]
fn reconnecting_updates_bridge_snapshot() {
    let mut engine = TranslationEngine::new();
    engine.status.0 = PipelineState::Active;
    assert!(
        engine.apply_bridge_status_event(BridgeStatusEvent::Reconnecting {
            direction: "outbound".to_string(),
            attempt: 2,
        })
    );
    let snapshot = engine.status_snapshot();
    assert_eq!(
        snapshot.outbound_bridge,
        BridgeConnectionState::Reconnecting
    );
    assert_eq!(snapshot.outbound_reconnect_attempt, Some(2));
}

#[test]
fn ready_after_reconnect_resets_active_since() {
    let mut engine = TranslationEngine::new();
    engine.status.0 = PipelineState::Active;
    engine.outbound_side.active_since = Some(1_000);
    engine.outbound_side.bridge = BridgeConnectionState::Reconnecting;
    assert!(engine.apply_bridge_status_event(BridgeStatusEvent::Ready {
        direction: "outbound".to_string(),
        reconnected: true,
    }));
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_bridge, BridgeConnectionState::Ready);
    assert!(snapshot.outbound_active_since.unwrap() > 1_000);
}

#[test]
fn ready_ignored_when_pipeline_not_active() {
    let mut engine = TranslationEngine::new();
    engine.status.0 = PipelineState::Starting;
    assert!(!engine.apply_bridge_status_event(BridgeStatusEvent::Ready {
        direction: "outbound".to_string(),
        reconnected: false,
    }));
    assert_eq!(engine.outbound_side.bridge, BridgeConnectionState::Idle);
}

#[test]
fn snapshot_includes_mic_muted() {
    let engine = TranslationEngine::new();
    engine.mic_mute.set_muted(true);
    let snapshot = engine.status_snapshot();
    assert!(snapshot.mic_muted);
}

#[test]
fn reset_mic_mute_clears_flag() {
    let mut engine = TranslationEngine::new();
    engine.mic_mute.set_muted(true);
    engine.reset_mic_mute();
    assert!(!engine.mic_mute.is_muted());
}

#[test]
fn snapshot_exposes_audio_recovery_when_pipeline_off() {
    let mut engine = TranslationEngine::new();
    engine.outbound_side.audio_path = AudioPathPhase::Reconnecting {
        attempt: 2,
        next_action_ms: None,
    };
    engine.status.0 = PipelineState::Off;
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_audio, AudioConnectionState::Reconnecting);
    assert_eq!(snapshot.outbound_audio_reconnect_attempt, Some(2));
}

#[test]
fn snapshot_exposes_audio_lost_when_translate_errored() {
    let mut engine = TranslationEngine::new();
    engine.outbound_side.audio_path = AudioPathPhase::Lost { since_ms: 1 };
    engine.status.0 = PipelineState::Error;
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_audio, AudioConnectionState::Lost);
}

#[test]
fn recovery_success_keeps_reconnecting_until_confirmed() {
    let mut engine = TranslationEngine::new();
    engine.status.0 = PipelineState::Direct;
    engine.outbound_side.audio_path = AudioPathPhase::Reconnecting {
        attempt: 1,
        next_action_ms: Some(9_999_999),
    };

    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_audio, AudioConnectionState::Reconnecting);
    assert_eq!(snapshot.outbound_audio_reconnect_attempt, Some(1));
}

#[test]
fn snapshot_exposes_audio_reconnecting_when_direct() {
    let mut engine = TranslationEngine::new();
    engine.status.0 = PipelineState::Direct;
    engine.outbound_side.audio_path = AudioPathPhase::Reconnecting {
        attempt: 1,
        next_action_ms: None,
    };
    let snapshot = engine.status_snapshot();
    assert_eq!(snapshot.outbound_audio, AudioConnectionState::Reconnecting);
    assert_eq!(snapshot.outbound_audio_reconnect_attempt, Some(1));
}

#[test]
fn stop_direct_outbound_internal_resets_mic_mute() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut engine = TranslationEngine::new();
        engine.mic_mute.set_muted(true);
        engine.stop_direct_outbound_internal().await;
        assert!(!engine.mic_mute.is_muted());
    });
}

#[test]
fn snapshot_includes_speaker_muted() {
    let engine = TranslationEngine::new();
    engine.speaker_mute.set_muted(true);
    let snapshot = engine.status_snapshot();
    assert!(snapshot.speaker_muted);
}

#[test]
fn stop_direct_inbound_internal_resets_speaker_mute() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut engine = TranslationEngine::new();
        engine.speaker_mute.set_muted(true);
        engine.stop_direct_inbound_internal().await;
        assert!(!engine.speaker_mute.is_muted());
    });
}
