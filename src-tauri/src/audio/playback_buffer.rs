use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::resampler::resample_mono_to_rate;
use crate::config::INPUT_SAMPLE_RATE;

/// Playback-side jitter / wait tuning (P1).
#[derive(Debug, Clone)]
pub struct PlaybackBufferConfig {
    /// Hold last sample instead of silence within this window after PCM ingest.
    pub jitter: Duration,
    /// Block up to this long waiting for the next mpsc chunk before padding.
    pub recv_wait: Duration,
    /// After jitter expires, keep holding last sample while the stream is still active.
    pub stream_tail: Duration,
    poll_interval: Duration,
}

impl Default for PlaybackBufferConfig {
    fn default() -> Self {
        Self {
            jitter: Duration::from_millis(80),
            recv_wait: Duration::from_millis(20),
            stream_tail: Duration::from_millis(400),
            poll_interval: Duration::from_millis(2),
        }
    }
}

impl PlaybackBufferConfig {
    /// Custom voice outbound — longer wait/hold for bursty WS sub-chunks.
    pub fn clone_outbound() -> Self {
        Self {
            jitter: Duration::from_millis(120),
            recv_wait: Duration::from_millis(35),
            stream_tail: Duration::from_millis(500),
            poll_interval: Duration::from_millis(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackFillStatus {
    Ok,
    Disconnected,
}

/// Sample ring @ device playback rate — decouples mpsc message boundaries from DAC fill.
pub struct PlaybackRingBuffer {
    pending: VecDeque<i16>,
    config: PlaybackBufferConfig,
    source_rate: u32,
    device_rate: u32,
    last_pcm_at: Option<Instant>,
    last_sample: i16,
}

impl PlaybackRingBuffer {
    pub fn new(config: PlaybackBufferConfig, device_rate: u32) -> Self {
        Self {
            pending: VecDeque::new(),
            config,
            source_rate: INPUT_SAMPLE_RATE,
            device_rate: device_rate.max(1),
            last_pcm_at: None,
            last_sample: 0,
        }
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    /// Update DAC rate (e.g. Bluetooth HFP switch). Clears pending samples that
    /// were queued for the previous rate to avoid stretched/slow playback.
    pub fn set_device_rate(&mut self, device_rate: u32) {
        let device_rate = device_rate.max(1);
        if device_rate == self.device_rate {
            return;
        }
        self.device_rate = device_rate;
        self.pending.clear();
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn ingest_source_pcm(&mut self, samples: Vec<i16>) {
        if samples.is_empty() {
            return;
        }
        let device = if self.device_rate == self.source_rate {
            samples
        } else {
            resample_mono_to_rate(&samples, self.source_rate, self.device_rate)
        };
        self.ingest_device_pcm(device);
    }

    fn ingest_device_pcm(&mut self, samples: Vec<i16>) {
        if samples.is_empty() {
            return;
        }
        self.last_pcm_at = Some(Instant::now());
        if let Some(&s) = samples.last() {
            self.last_sample = s;
        }
        self.pending.extend(samples);
    }

    pub fn drain_rx(&mut self, rx: &mut mpsc::Receiver<Vec<i16>>) -> bool {
        let mut got = false;
        loop {
            match rx.try_recv() {
                Ok(samples) => {
                    self.ingest_source_pcm(samples);
                    got = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => return got,
            }
        }
        got
    }

    fn wait_for_rx(&mut self, rx: &mut mpsc::Receiver<Vec<i16>>) {
        let deadline = Instant::now() + self.config.recv_wait;
        while Instant::now() < deadline {
            if self.drain_rx(rx) {
                return;
            }
            std::thread::sleep(self.config.poll_interval);
        }
    }

    fn rx_disconnected(rx: &mut mpsc::Receiver<Vec<i16>>) -> bool {
        matches!(rx.try_recv(), Err(mpsc::error::TryRecvError::Disconnected))
    }

    /// Realtime-safe fill: drain available PCM and pad immediately — never sleep.
    ///
    /// Use from Core Audio / WASAPI **IO callbacks**. Blocking wait belongs only
    /// on a dedicated worker thread ([`ensure_device_samples_blocking`]).
    pub fn ensure_device_samples(
        &mut self,
        samples_needed: usize,
        rx: &mut mpsc::Receiver<Vec<i16>>,
    ) -> PlaybackFillStatus {
        self.drain_rx(rx);
        self.pad_to(samples_needed, rx)
    }

    /// Worker-thread fill: briefly poll for more PCM before padding (Windows render loop).
    pub fn ensure_device_samples_blocking(
        &mut self,
        samples_needed: usize,
        rx: &mut mpsc::Receiver<Vec<i16>>,
    ) -> PlaybackFillStatus {
        self.drain_rx(rx);
        if self.pending.len() < samples_needed {
            self.wait_for_rx(rx);
            self.drain_rx(rx);
        }
        self.pad_to(samples_needed, rx)
    }

    fn pad_to(
        &mut self,
        samples_needed: usize,
        rx: &mut mpsc::Receiver<Vec<i16>>,
    ) -> PlaybackFillStatus {
        if self.pending.len() < samples_needed {
            if Self::rx_disconnected(rx) && self.pending.is_empty() {
                return PlaybackFillStatus::Disconnected;
            }
            let pad = self.pad_sample();
            while self.pending.len() < samples_needed {
                self.pending.push_back(pad);
            }
        }
        PlaybackFillStatus::Ok
    }

    fn pad_sample(&self) -> i16 {
        let within_jitter = self
            .last_pcm_at
            .map(|t| t.elapsed() < self.config.jitter)
            .unwrap_or(false);
        let stream_active = self
            .last_pcm_at
            .map(|t| t.elapsed() < self.config.stream_tail)
            .unwrap_or(false);
        if within_jitter || stream_active {
            self.last_sample
        } else {
            0
        }
    }

    pub fn pop_device_samples(&mut self, count: usize) -> Vec<i16> {
        let n = count.min(self.pending.len());
        self.pending.drain(..n).collect()
    }

    /// Pop into a caller-owned buffer (avoids per-callback `Vec` allocation).
    pub fn pop_device_samples_into(&mut self, dest: &mut [i16]) -> usize {
        let n = dest.len().min(self.pending.len());
        for (i, sample) in self.pending.drain(..n).enumerate() {
            dest[i] = sample;
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_rx_burst_into_ring() {
        let (tx, mut rx) = mpsc::channel(8);
        tx.try_send(vec![1, 2, 3]).unwrap();
        tx.try_send(vec![4, 5]).unwrap();

        let mut ring = PlaybackRingBuffer::new(PlaybackBufferConfig::default(), 48_000);
        ring.drain_rx(&mut rx);
        assert_eq!(ring.len(), 5);
    }

    #[test]
    fn pads_with_last_sample_within_jitter() {
        let (_tx, mut rx) = mpsc::channel::<Vec<i16>>(8);
        let mut ring = PlaybackRingBuffer::new(PlaybackBufferConfig::default(), 48_000);
        ring.ingest_source_pcm(vec![100, 200, 300]);
        assert_eq!(
            ring.ensure_device_samples(6, &mut rx),
            PlaybackFillStatus::Ok
        );
        let out = ring.pop_device_samples(6);
        assert_eq!(out, vec![100, 200, 300, 300, 300, 300]);
    }

    #[test]
    fn realtime_fill_does_not_block_when_empty() {
        let (_tx, mut rx) = mpsc::channel::<Vec<i16>>(8);
        let mut ring = PlaybackRingBuffer::new(PlaybackBufferConfig::default(), 48_000);
        let start = Instant::now();
        assert_eq!(
            ring.ensure_device_samples(4, &mut rx),
            PlaybackFillStatus::Ok
        );
        assert!(start.elapsed() < Duration::from_millis(5));
        assert_eq!(ring.pop_device_samples(4), vec![0, 0, 0, 0]);
    }

    #[test]
    fn pop_device_samples_into_fills_prefix() {
        let mut ring = PlaybackRingBuffer::new(PlaybackBufferConfig::default(), 48_000);
        ring.ingest_source_pcm(vec![1, 2, 3, 4, 5]);
        let mut dest = [0i16; 3];
        assert_eq!(ring.pop_device_samples_into(&mut dest), 3);
        assert_eq!(dest, [1, 2, 3]);
        assert_eq!(ring.len(), 2);
    }

    #[test]
    fn clone_config_has_longer_recv_wait() {
        let d = PlaybackBufferConfig::default();
        let c = PlaybackBufferConfig::clone_outbound();
        assert!(c.recv_wait > d.recv_wait);
        assert!(c.jitter > d.jitter);
    }

    #[test]
    fn set_device_rate_clears_pending() {
        let mut ring = PlaybackRingBuffer::new(PlaybackBufferConfig::default(), 44_100);
        ring.ingest_source_pcm(vec![1, 2, 3, 4]);
        assert!(!ring.is_empty());
        ring.set_device_rate(16_000);
        assert_eq!(ring.device_rate(), 16_000);
        assert_eq!(ring.len(), 0);
    }
}
