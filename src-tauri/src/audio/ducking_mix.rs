//! Continuous original underlay under translated TTS (48 kHz mono i16).
//!
//! Passthrough clock drives output: each original frame is scaled by a fixed gain and
//! mixed with any queued TTS samples. No delay-align, no idle/duck envelope (those
//! caused audible pumping / clicks).

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuckingParams {
    pub enabled: bool,
    /// Fixed underlay gain while feature is on (0.0–0.5).
    pub gain: f32,
}

impl DuckingParams {
    pub fn from_config_fields(enabled: bool, gain: f32) -> Self {
        Self {
            enabled,
            gain: gain.clamp(0.0, 0.5),
        }
    }

    pub fn exclusive_tts_parity(&self) -> bool {
        !self.enabled || self.gain <= 0.0
    }
}

/// Result of queueing upsampled TTS into the underlay mixer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueTtsOutcome {
    Queued,
    /// Feature off / zero gain / empty chunk.
    Skipped,
    /// Queue at ~30s cap; newest chunk rejected so the head stays intact.
    DroppedOverCap,
}

impl EnqueueTtsOutcome {
    pub fn is_queued(self) -> bool {
        matches!(self, Self::Queued)
    }
}

/// Mixes continuous meeting floor under optional queued TTS.
pub struct DuckingMixer {
    params: DuckingParams,
    tts_q: VecDeque<i16>,
}

impl DuckingMixer {
    pub fn new(params: DuckingParams) -> Self {
        Self {
            params,
            tts_q: VecDeque::with_capacity(48_000),
        }
    }

    pub fn params(&self) -> DuckingParams {
        self.params
    }

    /// Queue upsampled TTS; drained by [`Self::render_with_original`].
    ///
    /// Never splits a chunk: partial accept would cut mid-phrase inside one TTS
    /// sub-chunk and sound like a jump to the next segment.
    ///
    /// Backpressure: ~30s cap; drop **newest whole
    /// chunk** so audio already queued/playing at the head is not truncated.
    pub fn enqueue_tts(&mut self, tts_48k: &[i16]) -> EnqueueTtsOutcome {
        if self.params.exclusive_tts_parity() || tts_48k.is_empty() {
            return EnqueueTtsOutcome::Skipped;
        }
        // ~30s @ 48 kHz — safety valve for bursty custom-voice/TTS; prefers continuity
        // of queued head over accepting unbounded lag.
        const MAX_Q: usize = 48_000 * 30;
        if self.tts_q.len().saturating_add(tts_48k.len()) > MAX_Q {
            tracing::warn!(
                queued_samples = self.tts_q.len(),
                dropped_samples = tts_48k.len(),
                max_samples = MAX_Q,
                "inbound underlay TTS queue full; dropping newest chunk (head preserved)"
            );
            return EnqueueTtsOutcome::DroppedOverCap;
        }
        self.tts_q.extend(tts_48k.iter().copied());
        EnqueueTtsOutcome::Queued
    }

    /// Continuous underlay: `out[i] = sat_add(tts_or_0, original[i] * gain)`.
    /// Returns `None` when feature is off (caller uses exclusive TTS path).
    pub fn render_with_original(&mut self, original_48k: &[i16]) -> Option<Vec<i16>> {
        if self.params.exclusive_tts_parity() {
            return None;
        }
        if original_48k.is_empty() {
            return None;
        }
        let gain = self.params.gain;
        let mut out = Vec::with_capacity(original_48k.len());
        for &orig in original_48k {
            let tts = self.tts_q.pop_front().unwrap_or(0);
            out.push(sat_add(tts, scale_i16(orig, gain)));
        }
        Some(out)
    }

    pub fn set_params(&mut self, params: DuckingParams) {
        let was_exclusive = self.params.exclusive_tts_parity();
        self.params = params;
        if self.params.exclusive_tts_parity() {
            self.tts_q.clear();
        } else if was_exclusive {
            // freshly enabled — start clean
            self.tts_q.clear();
        }
    }

    pub fn reset(&mut self) {
        self.tts_q.clear();
    }
}

fn scale_i16(sample: i16, gain: f32) -> i16 {
    let v = (sample as f32 * gain).round();
    v.clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn sat_add(a: i16, b: i16) -> i16 {
    (a as i32 + b as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_returns_none_and_ignores_enqueue() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(false, 0.18));
        m.enqueue_tts(&[1000; 16]);
        assert!(m.render_with_original(&[500; 16]).is_none());
        assert!(m.tts_q.is_empty());
    }

    #[test]
    fn zero_gain_exclusive_parity() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.0));
        m.enqueue_tts(&[1000; 8]);
        assert!(m.render_with_original(&[8000; 8]).is_none());
    }

    #[test]
    fn continuous_original_without_tts() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.5));
        let out = m.render_with_original(&[10_000; 4]).unwrap();
        assert_eq!(out, vec![5000; 4]);
    }

    #[test]
    fn mixes_queued_tts_over_continuous_original() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.5));
        m.enqueue_tts(&[1000, 1000]);
        let out = m.render_with_original(&[10_000, 10_000, 10_000]).unwrap();
        // first two: 1000+5000; third: 0+5000
        assert_eq!(out, vec![6000, 6000, 5000]);
        assert!(m.tts_q.is_empty());
    }

    #[test]
    fn saturation_clamps() {
        assert_eq!(sat_add(20_000, 20_000), i16::MAX);
        assert_eq!(sat_add(-20_000, -20_000), i16::MIN);
    }

    #[test]
    fn set_params_to_off_clears_queue() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.2));
        m.enqueue_tts(&[1, 2, 3]);
        m.set_params(DuckingParams::from_config_fields(false, 0.2));
        assert!(m.tts_q.is_empty());
    }

    #[test]
    fn enqueue_over_cap_keeps_playing_head_drops_newest() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.18));
        const SEC: usize = 48_000;
        // Fill near the 30s cap with marker 1 at the head (currently playing).
        assert!(m.enqueue_tts(&vec![1; SEC * 30]).is_queued());
        assert_eq!(m.tts_q.len(), SEC * 30);
        assert_eq!(m.tts_q.front(), Some(&1));

        // Overflow must not truncate the head mid-phrase.
        assert_eq!(
            m.enqueue_tts(&vec![9; SEC]),
            EnqueueTtsOutcome::DroppedOverCap
        );
        assert_eq!(m.tts_q.len(), SEC * 30);
        assert_eq!(m.tts_q.front(), Some(&1));
        assert!(!m.tts_q.iter().any(|&s| s == 9));
    }

    #[test]
    fn enqueue_never_splits_chunk_when_near_cap() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.18));
        const MAX_Q: usize = 48_000 * 30;
        assert!(m.enqueue_tts(&vec![1; MAX_Q - 10]).is_queued());
        // 100-sample chunk does not fit wholly — must not take a 10-sample prefix.
        assert_eq!(m.enqueue_tts(&[7; 100]), EnqueueTtsOutcome::DroppedOverCap);
        assert_eq!(m.tts_q.len(), MAX_Q - 10);
        assert!(!m.tts_q.iter().any(|&s| s == 7));
    }

    #[test]
    fn tts_underruns_when_remainder_not_enqueued_before_clock() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.01));
        let frame = 4_800; // 100 ms @ 48 kHz
        assert!(m.enqueue_tts(&vec![5_000; frame]).is_queued());
        let playing = m.render_with_original(&vec![0; frame]).unwrap();
        assert!(playing.iter().any(|&s| s != 0));
        let gap = m.render_with_original(&vec![0; frame]).unwrap();
        assert!(
            gap.iter().all(|&s| s == 0),
            "underrun gap while remainder still on bridge"
        );
        assert!(m.enqueue_tts(&vec![6_000; frame]).is_queued());
        let resumed = m.render_with_original(&vec![0; frame]).unwrap();
        assert!(resumed.iter().any(|&s| s != 0));
    }

    #[test]
    fn set_params_gain_only_keeps_tts_queue() {
        let mut m = DuckingMixer::new(DuckingParams::from_config_fields(true, 0.18));
        m.enqueue_tts(&[1, 2, 3, 4]);
        m.set_params(DuckingParams::from_config_fields(true, 0.3));
        assert_eq!(m.tts_q.len(), 4);
        assert_eq!(m.params().gain, 0.3);
    }

    #[test]
    fn from_config_fields_clamps_gain() {
        let hi = DuckingParams::from_config_fields(true, 0.9);
        let lo = DuckingParams::from_config_fields(true, -0.1);
        assert_eq!(hi.gain, 0.5);
        assert_eq!(lo.gain, 0.0);
    }
}
