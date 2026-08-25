//! Coalesce Soniox TTS PCM before sending to the mux.
//!
//! Hold the first mux emit of each generation until preroll (`PLAYOUT_JITTER_MS`)
//! *and* a second audio packet (or `audio_end` / `terminated` for a short clip).
//! Then concatenate until ~80 ms and pass samples through unmodified.

use crate::audio::PlaybackPcmChunk;

use super::config::{playout_jitter_samples, SONIOX_PCM_COALESCE_MIN_SAMPLES};
#[cfg(test)]
use super::config::{PLAYOUT_JITTER_MS, TTS_SAMPLE_RATE};

pub struct PcmCoalesce {
    buf: Vec<i16>,
    min: usize,
    jitter: usize,
    started: bool,
    pushes: u32,
}

impl PcmCoalesce {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            min: SONIOX_PCM_COALESCE_MIN_SAMPLES,
            jitter: playout_jitter_samples(),
            started: false,
            pushes: 0,
        }
    }

    fn ready_to_start(&self) -> bool {
        if self.started {
            return true;
        }
        if self.jitter == 0 {
            return true;
        }
        // First audio packet is often already ≥ jitter. Wait for a second
        // push so packet 2 is queued before the DAC starts; `finish_segment`
        // covers single-packet clips.
        self.pushes >= 2 && self.buf.len() >= self.jitter
    }

    pub fn push(&mut self, samples: Vec<i16>) -> Vec<PlaybackPcmChunk> {
        if samples.is_empty() {
            return Vec::new();
        }
        self.buf.extend(samples);
        self.pushes = self.pushes.saturating_add(1);
        if !self.ready_to_start() {
            return Vec::new();
        }
        self.started = true;
        if self.buf.len() < self.min {
            return Vec::new();
        }
        vec![PlaybackPcmChunk::new(std::mem::take(&mut self.buf))]
    }

    pub fn finish_segment(&mut self) -> Vec<PlaybackPcmChunk> {
        self.started = false;
        self.pushes = 0;
        if self.buf.is_empty() {
            return Vec::new();
        }
        vec![PlaybackPcmChunk::new(std::mem::take(&mut self.buf))]
    }

    pub fn reset(&mut self) -> Vec<PlaybackPcmChunk> {
        self.finish_segment()
    }
}

impl Default for PcmCoalesce {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatten(chunks: Vec<PlaybackPcmChunk>) -> Vec<i16> {
        chunks.into_iter().flat_map(|c| c.samples).collect()
    }

    fn coalesce_no_preroll() -> PcmCoalesce {
        let mut p = PcmCoalesce::new();
        p.jitter = 0;
        p
    }

    #[test]
    fn holds_until_coalesce_or_done() {
        let mut p = coalesce_no_preroll();
        p.min = 4;
        assert!(p.push(vec![1, 2, 3]).is_empty());
        let out = p.finish_segment();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].samples, vec![1, 2, 3]);
    }

    #[test]
    fn emits_once_coalesced() {
        let mut p = coalesce_no_preroll();
        p.min = 4;
        let out = p.push(vec![1, 2, 3, 4, 5]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].samples, vec![1, 2, 3, 4, 5]);
        assert!(p.finish_segment().is_empty());
    }

    /// Regression lock: `PcmCoalesce::new()` must keep 200 ms preroll *and* wait
    /// for a second audio packet (or `audio_end` / `terminated`). Sample-count
    /// jitter alone is a no-op when the first packet is already ≥200 ms.
    #[test]
    fn default_coalesce_does_not_start_playout_on_first_packet_alone() {
        let mut p = PcmCoalesce::new();
        assert_eq!(PLAYOUT_JITTER_MS, 200);
        assert_eq!(p.jitter, (TTS_SAMPLE_RATE as usize * 200) / 1000);
        assert_eq!(p.min, SONIOX_PCM_COALESCE_MIN_SAMPLES);

        let first = vec![7i16; 5872];
        assert!(
            p.push(first.clone()).is_empty(),
            "must not start the DAC on packet 1 even when it exceeds PLAYOUT_JITTER_MS"
        );

        let second = vec![9i16; SONIOX_PCM_COALESCE_MIN_SAMPLES];
        let heard = flatten(p.push(second.clone()));
        let mut expected = first;
        expected.extend_from_slice(&second);
        assert_eq!(heard, expected);

        assert!(p.finish_segment().is_empty());
        assert!(
            p.push(vec![3i16; 5872]).is_empty(),
            "next generation must preroll again"
        );
        assert_eq!(
            flatten(p.finish_segment()),
            vec![3i16; 5872],
            "short / single-packet clip must play on audio_end / terminated"
        );
    }

    #[test]
    fn first_packet_larger_than_jitter_still_waits_for_second_push() {
        let mut p = PcmCoalesce::new();
        p.jitter = 8;
        p.min = 4;
        let first: Vec<i16> = (0..20).collect();
        assert!(
            p.push(first.clone()).is_empty(),
            "must not start the DAC on packet 1 alone"
        );
        let second: Vec<i16> = (20..28).collect();
        let out = flatten(p.push(second));
        let mut expected: Vec<i16> = (0..20).collect();
        expected.extend(20..28);
        assert_eq!(out, expected);
    }

    #[test]
    fn short_single_packet_generation_plays_on_done() {
        let mut p = PcmCoalesce::new();
        p.jitter = 8;
        p.min = 4;
        assert!(p.push(vec![1, 2, 3]).is_empty());
        assert_eq!(flatten(p.finish_segment()), vec![1, 2, 3]);
    }

    #[test]
    fn next_generation_prerolls_again() {
        let mut p = PcmCoalesce::new();
        p.jitter = 8;
        p.min = 1;
        let _ = p.push(vec![1; 10]);
        let _ = p.push(vec![2; 10]);
        let _ = p.finish_segment();
        assert!(
            p.push(vec![3; 10]).is_empty(),
            "new AppendDelta generation must preroll again"
        );
    }
}
