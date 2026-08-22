use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::INPUT_SAMPLE_RATE;

/// Default crossfade length for outbound clone playback polish.
pub const DEFAULT_CROSSFADE_MS: u32 = 8;

/// Matches `PCM_JITTER` in platform playback backends.
pub const PCM_CROSSFADE_GAP_MS: u64 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmChunkBoundary {
    /// Mid-generation ElevenLabs audio delta — skip crossfade when flush-aware.
    Continuation,
    /// EL `isFinal` or standalone provider chunk.
    SegmentEnd,
}

#[derive(Debug, Clone)]
pub struct PlaybackPcmChunk {
    pub samples: Vec<i16>,
    pub boundary: PcmChunkBoundary,
}

impl PlaybackPcmChunk {
    pub fn continuation(samples: Vec<i16>) -> Self {
        Self {
            samples,
            boundary: PcmChunkBoundary::Continuation,
        }
    }

    pub fn segment_end(samples: Vec<i16>) -> Self {
        Self {
            samples,
            boundary: PcmChunkBoundary::SegmentEnd,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackCrossfadeOptions {
    pub crossfade_ms: u32,
    pub flush_aware: bool,
}

pub struct PcmCrossfadeMixer {
    crossfade_samples: usize,
    gap_threshold: Duration,
    hold_tail: Vec<i16>,
    last_emit_at: Option<Instant>,
    flush_aware: bool,
    prev_segment_end: bool,
}

impl PcmCrossfadeMixer {
    pub fn new(options: &PlaybackCrossfadeOptions) -> Self {
        let crossfade_ms = resolve_crossfade_ms(options.crossfade_ms);
        Self {
            crossfade_samples: crossfade_samples_for_ms(crossfade_ms, INPUT_SAMPLE_RATE),
            gap_threshold: Duration::from_millis(PCM_CROSSFADE_GAP_MS),
            hold_tail: Vec::new(),
            last_emit_at: None,
            flush_aware: options.flush_aware,
            prev_segment_end: false,
        }
    }

    pub fn push_chunk(&mut self, chunk: PlaybackPcmChunk) -> Vec<i16> {
        if self.crossfade_samples == 0 {
            return chunk.samples;
        }

        if chunk.samples.is_empty() {
            if chunk.boundary == PcmChunkBoundary::SegmentEnd {
                self.prev_segment_end = true;
            }
            return Vec::new();
        }

        // P2: EL sub-chunks within one generation — no tail hold (ring buffer handles gaps).
        if self.flush_aware
            && chunk.boundary == PcmChunkBoundary::Continuation
            && !self.prev_segment_end
        {
            let mut out: Vec<i16> = self.hold_tail.drain(..).collect();
            out.extend(chunk.samples);
            self.last_emit_at = Some(Instant::now());
            return out;
        }

        let gap = self.gap_since_last_emit();
        let mut out = Vec::new();

        if self.hold_tail.is_empty() {
            self.append_with_hold(chunk.samples, &mut out);
        } else if self.should_crossfade(gap) {
            self.crossfade_into(&chunk.samples, &mut out);
        } else {
            out.append(&mut self.hold_tail);
            self.append_with_hold(chunk.samples, &mut out);
        }

        self.prev_segment_end = chunk.boundary == PcmChunkBoundary::SegmentEnd;
        self.last_emit_at = Some(Instant::now());
        out
    }

    pub fn flush(&mut self) -> Vec<i16> {
        let tail = std::mem::take(&mut self.hold_tail);
        self.prev_segment_end = false;
        self.last_emit_at = None;
        tail
    }

    pub fn reset(&mut self) {
        self.hold_tail.clear();
        self.prev_segment_end = false;
        self.last_emit_at = None;
    }

    fn gap_since_last_emit(&self) -> Duration {
        self.last_emit_at
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    fn should_crossfade(&self, gap: Duration) -> bool {
        if gap >= self.gap_threshold {
            return false;
        }
        if !self.flush_aware {
            return true;
        }
        self.prev_segment_end
    }

    fn append_with_hold(&mut self, mut samples: Vec<i16>, out: &mut Vec<i16>) {
        let n = self.crossfade_samples;
        if samples.len() <= n {
            self.hold_tail.append(&mut samples);
            return;
        }
        let split = samples.len() - n;
        out.extend_from_slice(&samples[..split]);
        self.hold_tail = samples.split_off(split);
    }

    fn crossfade_into(&mut self, chunk: &[i16], out: &mut Vec<i16>) {
        let overlap = self
            .crossfade_samples
            .min(self.hold_tail.len())
            .min(chunk.len());
        if overlap == 0 {
            out.append(&mut self.hold_tail);
            self.append_with_hold(chunk.to_vec(), out);
            return;
        }

        for (i, (&a, &b)) in self
            .hold_tail
            .iter()
            .zip(chunk.iter())
            .take(overlap)
            .enumerate()
        {
            let t = i as f32 / overlap as f32;
            let gain_b = sin_sq_half(t);
            let gain_a = 1.0 - gain_b;
            let mixed = a as f32 * gain_a + b as f32 * gain_b;
            out.push(mixed.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
        }
        self.hold_tail.clear();

        if chunk.len() > overlap {
            self.append_with_hold(chunk[overlap..].to_vec(), out);
        }
    }
}

fn sin_sq_half(t: f32) -> f32 {
    let s = (std::f32::consts::FRAC_PI_2 * t).sin();
    s * s
}

fn crossfade_samples_for_ms(ms: u32, sample_rate: u32) -> usize {
    if ms == 0 {
        return 0;
    }
    ((sample_rate as u64 * ms as u64) / 1000).max(1) as usize
}

/// Adapts legacy 24 kHz `Vec<i16>` bridge PCM into [`PlaybackPcmChunk`] (continuation).
pub fn spawn_pcm24k_adapter(
    cancel: CancellationToken,
    mut pcm_rx: mpsc::Receiver<Vec<i16>>,
    pcm_drops: Arc<std::sync::atomic::AtomicU64>,
) -> mpsc::Receiver<PlaybackPcmChunk> {
    let (tx, rx) = mpsc::channel(super::PLAYBACK_PCM_CHANNEL_DEPTH);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                msg = pcm_rx.recv() => {
                    match msg {
                        Some(samples) => {
                            let _ = super::try_send_pcm_bounded(
                                &tx,
                                PlaybackPcmChunk::continuation(samples),
                                &pcm_drops,
                            );
                        }
                        None => break,
                    }
                }
            }
        }
    });
    rx
}

fn resolve_crossfade_ms(configured: u32) -> u32 {
    if let Ok(raw) = std::env::var("MEETRAL_PCM_CROSSFADE_MS") {
        if let Ok(parsed) = raw.parse::<u32>() {
            return parsed.clamp(0, 20);
        }
    }
    configured.clamp(0, 20)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixer(flush_aware: bool) -> PcmCrossfadeMixer {
        PcmCrossfadeMixer::new(&PlaybackCrossfadeOptions {
            crossfade_ms: 8,
            flush_aware,
        })
    }

    fn samples(len: usize, fill: i16) -> Vec<i16> {
        vec![fill; len]
    }

    #[test]
    fn short_first_chunk_held_until_second() {
        let mut m = mixer(false);
        let n = m.crossfade_samples;
        assert!(m
            .push_chunk(PlaybackPcmChunk::continuation(samples(n / 2, 1000)))
            .is_empty());
        let out = m.push_chunk(PlaybackPcmChunk::continuation(samples(n, 2000)));
        assert!(!out.is_empty());
    }

    #[test]
    fn gap_over_threshold_hard_join() {
        let mut m = mixer(false);
        let n = m.crossfade_samples;
        let _ = m.push_chunk(PlaybackPcmChunk::continuation(samples(n * 2, 1000)));
        m.last_emit_at = Some(Instant::now() - Duration::from_millis(100));
        let out = m.push_chunk(PlaybackPcmChunk::continuation(samples(n * 2, 2000)));
        assert!(!out.is_empty());
        assert!(out.windows(2).any(|w| w[0] == 1000 && w[1] == 2000));
    }

    #[test]
    fn flush_aware_continuation_passthrough_without_hold() {
        let mut m = mixer(true);
        let n = m.crossfade_samples;
        let out = m.push_chunk(PlaybackPcmChunk::continuation(samples(n * 2, 1000)));
        assert_eq!(out.len(), n * 2);
        assert!(m.hold_tail.is_empty());
        m.last_emit_at = Some(Instant::now());
        let out2 = m.push_chunk(PlaybackPcmChunk::continuation(samples(n * 2, 2000)));
        assert_eq!(out2.len(), n * 2);
        assert!(out2.iter().all(|&s| s == 2000));
    }

    #[test]
    fn flush_aware_crossfades_after_segment_end() {
        let mut m = mixer(true);
        let n = m.crossfade_samples;
        let _ = m.push_chunk(PlaybackPcmChunk::segment_end(samples(n * 2, 1000)));
        m.last_emit_at = Some(Instant::now());
        let out = m.push_chunk(PlaybackPcmChunk::continuation(samples(n * 2, -1000)));
        assert!(!out.is_empty());
        let has_blend = out.iter().any(|&s| s > -1000 && s < 1000);
        assert!(has_blend);
    }

    #[test]
    fn flush_emits_hold_tail() {
        let mut m = mixer(false);
        let n = m.crossfade_samples;
        assert!(m
            .push_chunk(PlaybackPcmChunk::continuation(samples(n / 2, 500)))
            .is_empty());
        let tail = m.flush();
        assert_eq!(tail.len(), n / 2);
    }

    #[test]
    fn reset_clears_state() {
        let mut m = mixer(false);
        let _ = m.push_chunk(PlaybackPcmChunk::continuation(samples(10, 1)));
        m.reset();
        assert!(m.hold_tail.is_empty());
        assert!(!m.prev_segment_end);
    }
}
