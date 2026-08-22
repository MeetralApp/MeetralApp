use crate::audio::{CaptureHandle, CaptureHeartbeat};
use crate::providers::shared::live::LiveBridgeHandle;
use crate::runtime::voice_runtime::stop_tts_session;

use super::{InboundPipeline, InboundProviderTts};

/// Async-teardown parts taken from `InboundPipeline` synchronously so the
/// engine guard can be dropped before any `.await` runs (lock_scope pattern:
/// short lock → take → drop guard → teardown → re-lock → publish).
pub(crate) struct InboundTeardown {
    bridge: Option<LiveBridgeHandle>,
    provider_tts: Option<InboundProviderTts>,
}

/// Close inbound TTS WebSockets and abort the STT bridge — engine-lock free.
pub(crate) async fn teardown_inbound_parts(mut parts: InboundTeardown) {
    if let Some(mut tts) = parts.provider_tts.take() {
        // No Flush/Reset preamble: the session is stopped immediately below,
        // and pre-stop commands only race the already-closing worker channel.
        if let Some(session) = tts.provider_session.take() {
            stop_tts_session(session, "inbound provider tts").await;
        }
        if let Some(session) = tts.el_session.take() {
            stop_tts_session(session, "inbound elevenlabs").await;
        }
    }
    // Abort immediately — live transcript was already flushed by TranslationEngine.
    // Graceful bridge stop can hang (provider finished-wait / select bugs) and stalls UI.
    if let Some(bridge) = parts.bridge.take() {
        bridge.abort().await;
    }
}

impl InboundPipeline {
    pub async fn stop(&mut self) {
        let (parts, capture) = self.take_teardown_parts();
        teardown_inbound_parts(parts).await;
        if let Some(handle) = capture {
            crate::runtime::direct_relay::join_capture_handle(handle).await;
        }
    }

    /// Take every async-teardown part and reset scalar state; callers run
    /// [`teardown_inbound_parts`] (and the capture join) after dropping the
    /// engine guard.
    pub(crate) fn take_teardown_parts(&mut self) -> (InboundTeardown, Option<CaptureHandle>) {
        let parts = InboundTeardown {
            bridge: self.bridge.take(),
            provider_tts: self.provider_tts.take(),
        };
        let capture = self.capture.take();
        self.audio_mode = None;
        self.capture_device_id = None;
        self.playback_device = None;
        self.heartbeat = CaptureHeartbeat::new();
        self.started_at_ms = None;
        self.capture_attach_tx = None;
        self.ducking_params = None;
        (parts, capture)
    }

    /// Tear down inbound session; return capture handle for join outside engine lock.
    pub async fn stop_take_capture(&mut self) -> Option<CaptureHandle> {
        let (parts, capture) = self.take_teardown_parts();
        teardown_inbound_parts(parts).await;
        capture
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    /// The teardown split must take parts synchronously so callers can drop
    /// the engine lock before awaiting (lock-across-await regression).
    #[tokio::test]
    async fn take_teardown_parts_clears_pipeline_and_defers_capture_join() {
        let gate = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let gate_thread = gate.clone();
        let thread = std::thread::spawn(move || {
            while !gate_thread.load(Ordering::Relaxed) {
                std::thread::yield_now();
            }
        });
        let mut pipeline = InboundPipeline::new();
        pipeline.capture = Some(CaptureHandle::new(
            stop,
            thread,
            "fake-device".to_string(),
            CaptureHeartbeat::new(),
        ));

        let (parts, capture) = pipeline.take_teardown_parts();
        assert!(pipeline.capture.is_none());
        assert!(!pipeline.is_active());

        teardown_inbound_parts(parts).await;

        gate.store(true, Ordering::Relaxed);
        crate::runtime::direct_relay::join_capture_handle(capture.expect("capture taken")).await;
    }
}
