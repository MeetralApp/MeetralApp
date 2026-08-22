/// Platform-agnostic PCM conversion helpers shared by Windows (WASAPI) and macOS (Core Audio).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmSampleFormat {
    Float32,
    Int16,
}

pub fn bytes_to_mono_i16(bytes: &[u8], channels: usize, format: PcmSampleFormat) -> Vec<i16> {
    let mut out = Vec::new();
    bytes_to_mono_i16_into(bytes, channels, format, &mut out);
    out
}

pub fn bytes_to_mono_i16_into(
    bytes: &[u8],
    channels: usize,
    format: PcmSampleFormat,
    out: &mut Vec<i16>,
) {
    let channels = channels.max(1);
    out.clear();
    match format {
        PcmSampleFormat::Float32 => {
            out.reserve(bytes.len() / (4 * channels));
            let mut floats = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks_exact(4) {
                floats.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            for frame in floats.chunks(channels) {
                let sum: f32 = frame.iter().sum();
                let avg = sum / channels as f32;
                out.push((avg.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            }
        }
        PcmSampleFormat::Int16 => {
            out.reserve(bytes.len() / (2 * channels));
            let mut samples = Vec::with_capacity(bytes.len() / 2);
            for chunk in bytes.chunks_exact(2) {
                samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
            }
            for frame in samples.chunks(channels) {
                let sum: i32 = frame.iter().map(|s| *s as i32).sum();
                out.push((sum / channels as i32) as i16);
            }
        }
    }
}

pub fn mono_i16_to_device_bytes(
    samples: &[i16],
    channels: u16,
    format: PcmSampleFormat,
) -> Vec<u8> {
    let mut out = Vec::new();
    mono_i16_to_device_bytes_into(samples, channels, format, &mut out);
    out
}

pub fn mono_i16_to_device_bytes_into(
    samples: &[i16],
    channels: u16,
    format: PcmSampleFormat,
    out: &mut Vec<u8>,
) {
    let channels = channels.max(1) as usize;
    out.clear();
    match format {
        PcmSampleFormat::Float32 => {
            out.reserve(samples.len() * channels * 4);
            for sample in samples {
                let f = *sample as f32 / i16::MAX as f32;
                let bytes = f.to_le_bytes();
                for _ in 0..channels {
                    out.extend_from_slice(&bytes);
                }
            }
        }
        PcmSampleFormat::Int16 => {
            out.reserve(samples.len() * channels * 2);
            for sample in samples {
                let bytes = sample.to_le_bytes();
                for _ in 0..channels {
                    out.extend_from_slice(&bytes);
                }
            }
        }
    }
}

/// Samples below this level are treated as unused channels (e.g. BlackHole 16ch with stereo input).
const FLOAT_SILENCE_THRESHOLD: f32 = 1e-7;

fn downmix_float_frame_to_f32(frame: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let mut active = 0u32;
    for &s in frame {
        if s.abs() > FLOAT_SILENCE_THRESHOLD {
            sum += s;
            active += 1;
        }
    }
    let divisor = if active == 0 { 1.0 } else { active as f32 };
    (sum / divisor).clamp(-1.0, 1.0)
}

fn downmix_float_frame_to_i16(frame: &[f32]) -> i16 {
    (downmix_float_frame_to_f32(frame) * i16::MAX as f32) as i16
}

/// Fast interleaved mono/stereo average; sparse threshold path for >2ch (BlackHole).
pub fn float_buffer_to_mono_f32_into(data: &[f32], channels: usize, out: &mut Vec<f32>) {
    let channels = channels.max(1);
    out.clear();
    out.reserve(data.len() / channels);
    match channels {
        1 => out.extend_from_slice(data),
        2 => {
            for chunk in data.chunks_exact(2) {
                out.push((chunk[0] + chunk[1]) * 0.5);
            }
            if data.len() % 2 == 1 {
                if let Some(&last) = data.last() {
                    out.push(last);
                }
            }
        }
        _ => {
            for frame in data.chunks(channels) {
                out.push(downmix_float_frame_to_f32(frame));
            }
        }
    }
}

pub fn float_channel_buffers_to_mono_f32_into(
    channels: &[(*const f32, usize)],
    out: &mut Vec<f32>,
    frame_scratch: &mut Vec<f32>,
) {
    out.clear();
    if channels.is_empty() {
        return;
    }
    let frame_count = channels.iter().map(|(_, len)| *len).max().unwrap_or(0);
    out.reserve(frame_count);
    let n_ch = channels.len();
    match n_ch {
        1 => {
            let (ptr, len) = channels[0];
            // SAFETY: caller guarantees `ptr` is valid for `len` aligned `f32` values
            // for the duration of this call (null only when `len == 0`).
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            out.extend_from_slice(slice);
        }
        2 => {
            let (p0, l0) = channels[0];
            let (p1, l1) = channels[1];
            // SAFETY: caller guarantees each `(ptr, len)` is a valid aligned `f32` buffer
            // for this call's lifetime (null only when `len == 0`).
            let ch0 = unsafe { std::slice::from_raw_parts(p0, l0) };
            let ch1 = unsafe { std::slice::from_raw_parts(p1, l1) };
            for frame in 0..frame_count {
                let a = ch0.get(frame).copied().unwrap_or(0.0);
                let b = ch1.get(frame).copied().unwrap_or(0.0);
                out.push((a + b) * 0.5);
            }
        }
        _ => {
            for frame in 0..frame_count {
                frame_scratch.clear();
                for &(ptr, len) in channels {
                    // SAFETY: caller guarantees each `(ptr, len)` is a valid aligned `f32`
                    // buffer for this call's lifetime (null only when `len == 0`).
                    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
                    if let Some(&s) = slice.get(frame) {
                        frame_scratch.push(s);
                    }
                }
                out.push(downmix_float_frame_to_f32(frame_scratch));
            }
        }
    }
}

pub fn float_buffer_to_mono_i16(data: &[f32], channels: usize) -> Vec<i16> {
    let mut out = Vec::new();
    float_buffer_to_mono_i16_into(data, channels, &mut out);
    out
}

pub fn float_buffer_to_mono_i16_into(data: &[f32], channels: usize, out: &mut Vec<i16>) {
    let channels = channels.max(1);
    out.clear();
    out.reserve(data.len() / channels);
    match channels {
        1 => {
            for &s in data {
                out.push((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            }
        }
        2 => {
            for chunk in data.chunks_exact(2) {
                let avg = (chunk[0] + chunk[1]) * 0.5;
                out.push((avg.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            }
        }
        _ => {
            for frame in data.chunks(channels) {
                out.push(downmix_float_frame_to_i16(frame));
            }
        }
    }
}

/// Downmix non-interleaved float channel buffers (one buffer per channel) to mono i16.
pub fn float_channel_buffers_to_mono_i16(channels: &[&[f32]]) -> Vec<i16> {
    let mut out = Vec::new();
    let views: Vec<(*const f32, usize)> =
        channels.iter().map(|ch| (ch.as_ptr(), ch.len())).collect();
    let mut frame_scratch = Vec::with_capacity(channels.len());
    float_channel_buffers_to_mono_i16_into(&views, &mut out, &mut frame_scratch);
    out
}

pub fn float_channel_buffers_to_mono_i16_into(
    channels: &[(*const f32, usize)],
    out: &mut Vec<i16>,
    frame_scratch: &mut Vec<f32>,
) {
    out.clear();
    if channels.is_empty() {
        return;
    }
    let frame_count = channels.iter().map(|(_, len)| *len).max().unwrap_or(0);
    out.reserve(frame_count);
    let n_ch = channels.len();
    match n_ch {
        1 => {
            let (ptr, len) = channels[0];
            // SAFETY: caller guarantees `ptr` is valid for `len` aligned `f32` values
            // for the duration of this call (null only when `len == 0`).
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            for &s in slice {
                out.push((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            }
        }
        2 => {
            let (p0, l0) = channels[0];
            let (p1, l1) = channels[1];
            // SAFETY: caller guarantees each `(ptr, len)` is a valid aligned `f32` buffer
            // for this call's lifetime (null only when `len == 0`).
            let ch0 = unsafe { std::slice::from_raw_parts(p0, l0) };
            let ch1 = unsafe { std::slice::from_raw_parts(p1, l1) };
            for frame in 0..frame_count {
                let a = ch0.get(frame).copied().unwrap_or(0.0);
                let b = ch1.get(frame).copied().unwrap_or(0.0);
                let avg = (a + b) * 0.5;
                out.push((avg.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            }
        }
        _ => {
            for frame in 0..frame_count {
                frame_scratch.clear();
                for &(ptr, len) in channels {
                    // SAFETY: caller guarantees each `(ptr, len)` is a valid aligned `f32`
                    // buffer for this call's lifetime (null only when `len == 0`).
                    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
                    if let Some(&s) = slice.get(frame) {
                        frame_scratch.push(s);
                    }
                }
                out.push(downmix_float_frame_to_i16(frame_scratch));
            }
        }
    }
}

pub fn mono_i16_to_float_interleaved(samples: &[i16], channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    samples
        .iter()
        .flat_map(|sample| {
            let f = *sample as f32 / i16::MAX as f32;
            std::iter::repeat_n(f, channels)
        })
        .collect()
}

/// Write mono i16 as interleaved float32 into `dest` (length = samples.len() * channels).
pub fn write_mono_i16_as_float_interleaved(samples: &[i16], channels: usize, dest: &mut [f32]) {
    let channels = channels.max(1);
    let frames = samples.len().min(dest.len() / channels);
    for (frame, &sample) in samples.iter().take(frames).enumerate() {
        let f = sample as f32 / i16::MAX as f32;
        let base = frame * channels;
        dest[base..base + channels].fill(f);
    }
}

/// Write mono f32 as interleaved float32 into `dest`.
pub fn write_mono_f32_as_float_interleaved(samples: &[f32], channels: usize, dest: &mut [f32]) {
    let channels = channels.max(1);
    let frames = samples.len().min(dest.len() / channels);
    for (frame, &sample) in samples.iter().take(frames).enumerate() {
        let base = frame * channels;
        dest[base..base + channels].fill(sample);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_int16_roundtrip_single_channel() {
        let samples = vec![1000i16, -2000, 3000];
        let bytes = mono_i16_to_device_bytes(&samples, 1, PcmSampleFormat::Int16);
        let back = bytes_to_mono_i16(&bytes, 1, PcmSampleFormat::Int16);
        assert_eq!(samples, back);
    }

    #[test]
    fn stereo_downmix_to_mono() {
        let left = 1000i16.to_le_bytes();
        let right = 3000i16.to_le_bytes();
        let bytes: Vec<u8> = [left, right].into_iter().flatten().collect();
        let mono = bytes_to_mono_i16(&bytes, 2, PcmSampleFormat::Int16);
        assert_eq!(mono.len(), 1);
        assert_eq!(mono[0], 2000);
    }

    #[test]
    fn float_sparse_multichannel_downmix_ignores_silent_channels() {
        // BlackHole 16ch: stereo on first two channels, rest silent.
        let frame = {
            let mut f = [0.0f32; 16];
            f[0] = 1.0;
            f[1] = 1.0;
            f
        };
        let mono = float_buffer_to_mono_i16(&frame, 16);
        assert_eq!(mono.len(), 1);
        assert_eq!(mono[0], i16::MAX);
    }

    #[test]
    fn float_non_interleaved_sparse_channels_downmix() {
        let ch0 = [1.0f32, -1.0];
        let ch1 = [1.0f32, -1.0];
        let silent = [0.0f32; 2];
        let channels: Vec<&[f32]> = vec![&ch0, &ch1, &silent, &silent];
        let mono = float_channel_buffers_to_mono_i16(&channels);
        assert_eq!(mono.len(), 2);
        assert_eq!(mono[0], i16::MAX);
        assert!(mono[1] < -30_000);
    }

    #[test]
    fn float_stereo_fast_path_averages() {
        let data = [1.0f32, -1.0, 0.5, 0.5];
        let mut out = Vec::new();
        float_buffer_to_mono_f32_into(&data, 2, &mut out);
        assert_eq!(out, vec![0.0, 0.5]);
    }

    #[test]
    fn float_mono_passthrough() {
        let data = [0.25f32, -0.5];
        let mut out = Vec::new();
        float_buffer_to_mono_f32_into(&data, 1, &mut out);
        assert_eq!(out, data);
    }
}
