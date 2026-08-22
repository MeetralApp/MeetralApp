use tracing::info;

use crate::providers::elevenlabs::delivery::core::DeltaTracker;
use crate::providers::elevenlabs::delivery::modes::{NaturalState, SpeedState};
use crate::voice::shared::latency::TurnLatencySlot;

struct PhraseTiming {
    phrase_started_ms: u64,
    first_translated_ms: u64,
}

pub(crate) struct RelayState {
    pub(crate) tracker: DeltaTracker,
    pub(crate) speed: SpeedState,
    pub(crate) natural: NaturalState,
    flushes_per_turn: u32,
    phrase_timing: PhraseTiming,
}

impl RelayState {
    pub(crate) fn new() -> Self {
        Self {
            tracker: DeltaTracker::new(),
            speed: SpeedState::new(),
            natural: NaturalState::new(),
            flushes_per_turn: 0,
            phrase_timing: PhraseTiming {
                phrase_started_ms: 0,
                first_translated_ms: 0,
            },
        }
    }

    pub(crate) fn reset_phrase(&mut self, turn_latency: &TurnLatencySlot) {
        self.tracker.reset();
        self.speed.reset();
        self.natural.reset();
        self.phrase_timing.phrase_started_ms = 0;
        self.phrase_timing.first_translated_ms = 0;
        turn_latency.reset();
    }

    pub(crate) fn reanchor(&mut self, anchor: &str) {
        self.tracker.reanchor(anchor);
        self.speed.el_sent_anchor = anchor.to_string();
    }

    pub(crate) fn reset_turn_metrics(&mut self) {
        self.speed.revisions_debounced = 0;
        self.speed.major_rewrites = 0;
        self.flushes_per_turn = 0;
    }

    pub(crate) fn log_turn_metrics(&self) {
        info!(
            revisions_debounced = self.speed.revisions_debounced,
            major_rewrites = self.speed.major_rewrites,
            flushes_per_turn = self.flushes_per_turn,
            "sbpc-metrics"
        );
    }

    pub(crate) fn stamp_phrase_start(&mut self, turn_latency: &TurnLatencySlot) {
        if self.phrase_timing.phrase_started_ms == 0 {
            let now = crate::audio::monotonic_ms();
            self.phrase_timing.phrase_started_ms = now;
            self.phrase_timing.first_translated_ms = now;
            turn_latency.begin_turn(now, now);
        }
    }

    pub(crate) fn stamp_first_translated(&mut self, turn_latency: &TurnLatencySlot) {
        if self.phrase_timing.first_translated_ms == 0 {
            let now = crate::audio::monotonic_ms();
            self.phrase_timing.first_translated_ms = now;
            if self.phrase_timing.phrase_started_ms == 0 {
                self.phrase_timing.phrase_started_ms = now;
            }
            turn_latency.begin_turn(
                self.phrase_timing.phrase_started_ms,
                self.phrase_timing.first_translated_ms,
            );
        }
    }

    pub(crate) fn has_natural_pending(&self) -> bool {
        self.natural.has_pending(&self.tracker)
    }

    pub(crate) fn record_flush(&mut self) {
        self.flushes_per_turn += 1;
    }
}
