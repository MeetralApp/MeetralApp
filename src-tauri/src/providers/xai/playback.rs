//! Ordered PCM from two xAI sockets. Play seq 0, then 1, … with a short jitter
//! hold before the first playout so a pipelined socket can overlap TTFA.

use std::collections::HashMap;

use crate::audio::pcm_crossfade::{PcmChunkBoundary, PlaybackPcmChunk};

use super::config::{playout_jitter_samples, PCM_COALESCE_MIN_SAMPLES};

struct UnitBuf {
    samples: Vec<i16>,
    done: bool,
}

pub struct OrderedPlayback {
    next_seq: u64,
    units: HashMap<u64, UnitBuf>,
    started: bool,
    jitter_samples: usize,
    coalesce_min: usize,
}

impl OrderedPlayback {
    pub fn new() -> Self {
        Self {
            next_seq: 0,
            units: HashMap::new(),
            started: false,
            jitter_samples: playout_jitter_samples(),
            coalesce_min: PCM_COALESCE_MIN_SAMPLES,
        }
    }

    pub fn push_samples(&mut self, seq: u64, samples: Vec<i16>) -> Vec<PlaybackPcmChunk> {
        if samples.is_empty() {
            return Vec::new();
        }
        self.units.entry(seq).or_insert_with(|| UnitBuf {
            samples: Vec::new(),
            done: false,
        });
        if let Some(buf) = self.units.get_mut(&seq) {
            if buf.done {
                return Vec::new();
            }
            buf.samples.extend(samples);
        }
        self.drain()
    }

    pub fn mark_done(&mut self, seq: u64) -> Vec<PlaybackPcmChunk> {
        self.units
            .entry(seq)
            .or_insert_with(|| UnitBuf {
                samples: Vec::new(),
                done: false,
            })
            .done = true;
        self.drain()
    }

    pub fn reset(&mut self) -> Vec<PlaybackPcmChunk> {
        let mut out = Vec::new();
        if let Some(buf) = self.units.remove(&self.next_seq) {
            if !buf.samples.is_empty() {
                out.push(PlaybackPcmChunk {
                    samples: buf.samples,
                    boundary: PcmChunkBoundary::SegmentEnd,
                });
            }
        }
        self.units.clear();
        self.next_seq = 0;
        self.started = false;
        if out.is_empty() {
            out.push(PlaybackPcmChunk {
                samples: Vec::new(),
                boundary: PcmChunkBoundary::SegmentEnd,
            });
        }
        out
    }

    fn drain(&mut self) -> Vec<PlaybackPcmChunk> {
        let mut out = Vec::new();
        loop {
            let Some(buf) = self.units.get_mut(&self.next_seq) else {
                break;
            };

            if !self.started {
                if buf.samples.len() < self.jitter_samples && !buf.done {
                    break;
                }
                self.started = true;
            }

            if !buf.done && buf.samples.len() < self.coalesce_min {
                break;
            }

            if buf.done {
                let samples = std::mem::take(&mut buf.samples);
                out.push(PlaybackPcmChunk {
                    samples,
                    boundary: PcmChunkBoundary::SegmentEnd,
                });
                self.units.remove(&self.next_seq);
                self.next_seq += 1;
                continue;
            }

            let samples = std::mem::take(&mut buf.samples);
            out.push(PlaybackPcmChunk {
                samples,
                boundary: PcmChunkBoundary::Continuation,
            });
        }
        out
    }
}

impl Default for OrderedPlayback {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_first_unit_until_jitter_or_done() {
        let mut p = OrderedPlayback::new();
        p.jitter_samples = 8;
        p.coalesce_min = 4;
        let out = p.push_samples(0, vec![1, 2, 3]);
        assert!(out.is_empty(), "below jitter and not done");
        let out = p.mark_done(0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].boundary, PcmChunkBoundary::SegmentEnd);
        assert_eq!(out[0].samples, vec![1, 2, 3]);
    }

    #[test]
    fn does_not_play_seq1_before_seq0() {
        let mut p = OrderedPlayback::new();
        p.jitter_samples = 1;
        p.coalesce_min = 1;
        let mut heard = Vec::new();
        heard.extend(p.push_samples(1, vec![9, 9]));
        assert!(heard.is_empty());
        heard.extend(p.push_samples(0, vec![1, 2]));
        assert_eq!(heard[0].samples, vec![1, 2]);
        assert_eq!(heard[0].boundary, PcmChunkBoundary::Continuation);
        heard.extend(p.mark_done(0));
        heard.extend(p.mark_done(1));
        let samples: Vec<i16> = heard.iter().flat_map(|c| c.samples.iter().copied()).collect();
        assert_eq!(samples, vec![1, 2, 9, 9]);
        assert!(heard.iter().any(|c| c.boundary == PcmChunkBoundary::SegmentEnd));
    }

    #[test]
    fn reset_drops_queued_and_restarts_seq() {
        let mut p = OrderedPlayback::new();
        p.jitter_samples = 100;
        let _ = p.push_samples(0, vec![1, 2, 3]);
        let _ = p.reset();
        p.jitter_samples = 1;
        p.coalesce_min = 1;
        let out = p.push_samples(0, vec![7]);
        assert_eq!(out[0].samples, vec![7]);
    }
}
