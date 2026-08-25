//! Coalesce xAI `audio.delta` PCM before sending to the mux.
//!
//! Concatenate until ~80 ms, then pass samples through unmodified.

use crate::audio::pcm_crossfade::{PcmChunkBoundary, PlaybackPcmChunk};

use super::config::PCM_COALESCE_MIN_SAMPLES;

pub struct PcmCoalesce {
    buf: Vec<i16>,
    min: usize,
}

impl PcmCoalesce {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            min: PCM_COALESCE_MIN_SAMPLES,
        }
    }

    pub fn push(&mut self, samples: Vec<i16>) -> Vec<PlaybackPcmChunk> {
        if samples.is_empty() {
            return Vec::new();
        }
        self.buf.extend(samples);
        if self.buf.len() < self.min {
            return Vec::new();
        }
        vec![PlaybackPcmChunk {
            samples: std::mem::take(&mut self.buf),
            boundary: PcmChunkBoundary::Continuation,
        }]
    }

    pub fn finish_segment(&mut self) -> Vec<PlaybackPcmChunk> {
        vec![PlaybackPcmChunk {
            samples: std::mem::take(&mut self.buf),
            boundary: PcmChunkBoundary::SegmentEnd,
        }]
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

    fn max_adjacent_jump(samples: &[i16]) -> i32 {
        samples
            .windows(2)
            .map(|w| (w[1] as i32 - w[0] as i32).abs())
            .max()
            .unwrap_or(0)
    }

    fn smooth_chunk(len: usize, start: i16, end: i16) -> Vec<i16> {
        (0..len)
            .map(|i| {
                let t = i as f32 / ((len - 1).max(1) as f32);
                (start as f32 + (end as f32 - start as f32) * t).round() as i16
            })
            .collect()
    }

    #[test]
    fn holds_until_coalesce_or_done() {
        let mut p = PcmCoalesce::new();
        p.min = 4;
        assert!(p.push(vec![1, 2, 3]).is_empty());
        let out = p.finish_segment();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].boundary, PcmChunkBoundary::SegmentEnd);
        assert_eq!(out[0].samples, vec![1, 2, 3]);
    }

    #[test]
    fn emits_continuation_once_coalesced() {
        let mut p = PcmCoalesce::new();
        p.min = 4;
        let out = p.push(vec![1, 2, 3, 4, 5]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].boundary, PcmChunkBoundary::Continuation);
        assert_eq!(out[0].samples, vec![1, 2, 3, 4, 5]);
        let rest = p.finish_segment();
        assert!(rest[0].samples.is_empty());
        assert_eq!(rest[0].boundary, PcmChunkBoundary::SegmentEnd);
    }

    #[test]
    fn passthrough_does_not_rewrite_samples() {
        let mut p = PcmCoalesce::new();
        p.min = 1;
        let src = vec![30i16, 39, 8_000, -2_166, 0];
        let mut got = flatten(p.push(src.clone()));
        got.extend(flatten(p.finish_segment()));
        assert_eq!(got, src);
    }

    #[test]
    fn reset_drops_held_samples() {
        let mut p = PcmCoalesce::new();
        p.min = 100;
        let _ = p.push(vec![1, 2, 3]);
        let flushed = flatten(p.reset());
        assert_eq!(flushed, vec![1, 2, 3]);
        p.min = 1;
        let out = p.push(vec![7]);
        assert_eq!(out[0].samples, vec![7]);
    }

    #[test]
    fn coalesce_of_continuous_pcm_does_not_invent_clicks() {
        let mut p = PcmCoalesce::new();
        p.min = 64;
        let continuous = smooth_chunk(500, -2_000, 7_000);
        let mut heard = Vec::new();
        for piece in continuous.chunks(40) {
            heard.extend(flatten(p.push(piece.to_vec())));
        }
        heard.extend(flatten(p.finish_segment()));
        assert_eq!(heard, continuous);
        assert_eq!(max_adjacent_jump(&heard), max_adjacent_jump(&continuous));
    }
}
