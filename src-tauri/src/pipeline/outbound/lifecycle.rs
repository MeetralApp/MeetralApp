use std::sync::Arc;

use crate::audio::{CaptureHandle, CaptureHeartbeat};
use crate::providers::shared::live::LiveBridgeHandle;
use crate::runtime::voice_runtime::OutboundVoiceRuntime;

use super::{OutboundPipeline, OutboundSessionTasks, SESSION_TASK_JOIN_TIMEOUT};

/// Async-teardown parts taken from `OutboundPipeline` synchronously so the
/// engine guard can be dropped before any `.await` runs (lock_scope pattern:
/// short lock → take → drop guard → teardown → re-lock → publish).
pub(crate) struct OutboundTeardown {
    voice_runtime: Option<Arc<OutboundVoiceRuntime>>,
    bridge: Option<LiveBridgeHandle>,
    session_tasks: Option<OutboundSessionTasks>,
}

/// Abort bridges / stop TTS workers / join session tasks — engine-lock free.
pub(crate) async fn teardown_outbound_parts(parts: OutboundTeardown) {
    if let Some(runtime) = parts.voice_runtime {
        runtime.el_parent_cancel.cancel();
        crate::runtime::voice_runtime::stop_all_tts_workers(&runtime).await;
    }
    // Abort immediately — live transcript was already flushed by TranslationEngine.
    if let Some(bridge) = parts.bridge {
        bridge.abort().await;
    }
    if let Some(tasks) = parts.session_tasks {
        await_outbound_session_tasks(tasks).await;
    }
}

impl OutboundPipeline {
    pub async fn stop(&mut self) {
        let (parts, capture) = self.take_teardown_parts();
        teardown_outbound_parts(parts).await;
        if let Some(handle) = capture {
            crate::runtime::direct_relay::join_capture_handle(handle).await;
        }
    }

    /// Take every async-teardown part and reset scalar state; callers run
    /// [`teardown_outbound_parts`] (and the capture join) after dropping the
    /// engine guard.
    pub(crate) fn take_teardown_parts(&mut self) -> (OutboundTeardown, Option<CaptureHandle>) {
        let parts = OutboundTeardown {
            voice_runtime: self.voice_runtime.take(),
            bridge: self.bridge.take(),
            session_tasks: self.session_tasks.take(),
        };
        let capture = self.capture.take();
        self.audio_mode = None;
        self.capture_device_id = None;
        self.playback_device = None;
        self.heartbeat = CaptureHeartbeat::new();
        self.started_at_ms = None;
        self.capture_attach_tx = None;
        (parts, capture)
    }

    /// Tear down outbound session; return capture handle for join outside engine lock.
    pub async fn stop_take_capture(&mut self) -> Option<CaptureHandle> {
        let (parts, capture) = self.take_teardown_parts();
        teardown_outbound_parts(parts).await;
        capture
    }
}

pub(crate) async fn await_outbound_session_tasks(tasks: OutboundSessionTasks) {
    let OutboundSessionTasks {
        tts_worker,
        fanout,
        mux,
        idle_watcher,
    } = tasks;

    // Abort first (inbound-style), then brief join so stop does not stall the UI.
    if let Some(worker) = tts_worker {
        worker.abort();
        match tokio::time::timeout(SESSION_TASK_JOIN_TIMEOUT, worker).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) if e.is_cancelled() => {}
            Ok(Err(e)) => tracing::warn!("elevenlabs worker join error: {e}"),
            Err(_) => tracing::warn!("elevenlabs worker join timed out"),
        }
    }

    if let Some(mux) = mux {
        mux.abort();
        match tokio::time::timeout(SESSION_TASK_JOIN_TIMEOUT, mux).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) if e.is_cancelled() => {}
            Ok(Err(e)) => tracing::warn!("playback mux join error: {e}"),
            Err(_) => tracing::warn!("playback mux join timed out"),
        }
    }

    if let Some(idle) = idle_watcher {
        idle.abort();
        match tokio::time::timeout(SESSION_TASK_JOIN_TIMEOUT, idle).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) if e.is_cancelled() => {}
            Ok(Err(e)) => tracing::warn!("idle watcher join error: {e}"),
            Err(_) => tracing::warn!("idle watcher join timed out"),
        }
    }

    fanout.abort();
    match tokio::time::timeout(SESSION_TASK_JOIN_TIMEOUT, fanout).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) if e.is_cancelled() => {}
        Ok(Err(e)) => tracing::warn!("transcript fanout join error: {e}"),
        Err(_) => tracing::warn!("transcript fanout join timed out"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

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
        let mut pipeline = OutboundPipeline::new();
        pipeline.capture = Some(CaptureHandle::new(
            stop,
            thread,
            "fake-device".to_string(),
            CaptureHeartbeat::new(),
        ));

        let (parts, capture) = pipeline.take_teardown_parts();
        assert!(pipeline.capture.is_none());
        assert!(!pipeline.is_active());

        // No bridge/runtime/session tasks — teardown resolves immediately
        // while the fake capture thread is still gated.
        teardown_outbound_parts(parts).await;

        gate.store(true, Ordering::Relaxed);
        crate::runtime::direct_relay::join_capture_handle(capture.expect("capture taken")).await;
    }
}
