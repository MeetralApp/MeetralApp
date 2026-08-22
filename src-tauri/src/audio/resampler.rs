use crate::config::{INPUT_SAMPLE_RATE, OUTPUT_SAMPLE_RATE};

/// Upsample mono PCM from Gemini output (24 kHz) to playback rate (48 kHz).
pub fn upsample_24k_to_48k(samples: &[i16]) -> anyhow::Result<Vec<i16>> {
    if samples.is_empty() {
        return Ok(Vec::new());
    }

    let ratio = INPUT_SAMPLE_RATE as f64 / OUTPUT_SAMPLE_RATE as f64;
    if (ratio - 2.0).abs() < f64::EPSILON {
        return Ok(linear_upsample_x2(samples));
    }

    let out_len = ((samples.len() as f64) * ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = samples[idx.min(samples.len() - 1)] as f32;
        let b = samples[(idx + 1).min(samples.len() - 1)] as f32;
        let v = a + (b - a) * frac;
        out.push(v.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
    }

    Ok(out)
}

fn linear_upsample_x2(samples: &[i16]) -> Vec<i16> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for i in 0..samples.len() {
        let current = samples[i];
        let next = samples.get(i + 1).copied().unwrap_or(current);
        out.push(current);
        out.push(((current as i32 + next as i32) / 2) as i16);
    }
    out
}

pub fn resample_mono_to_rate(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    let mut out = Vec::new();
    resample_mono_to_rate_into(samples, from_rate, to_rate, &mut out);
    out
}

/// Resample into a reusable buffer (clears `out` first). Identity copies when rates match.
pub fn resample_mono_to_rate_into(
    samples: &[i16],
    from_rate: u32,
    to_rate: u32,
    out: &mut Vec<i16>,
) {
    out.clear();
    if samples.is_empty() {
        return;
    }
    if from_rate == to_rate {
        out.extend_from_slice(samples);
        return;
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let out_len = ((samples.len() as f64) * ratio).ceil() as usize;
    out.reserve(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = samples[idx.min(samples.len() - 1)] as f32;
        let b = samples[(idx + 1).min(samples.len() - 1)] as f32;
        let v = a + (b - a) * frac;
        out.push(v.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
    }
}

pub fn resample_for_provider_upload(samples: &[i16], target_rate: u32) -> Vec<i16> {
    if samples.is_empty() {
        return Vec::new();
    }
    if INPUT_SAMPLE_RATE == target_rate {
        return samples.to_vec();
    }

    let decimation = INPUT_SAMPLE_RATE / target_rate;
    if decimation > 1 && INPUT_SAMPLE_RATE == target_rate * decimation {
        return decimate_box_average(samples, decimation as usize);
    }

    resample_mono_to_rate(samples, INPUT_SAMPLE_RATE, target_rate)
}

fn decimate_box_average(samples: &[i16], factor: usize) -> Vec<i16> {
    let out_len = samples.len() / factor;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let base = i * factor;
        let sum: i32 = samples[base..base + factor].iter().map(|s| *s as i32).sum();
        out.push((sum / factor as i32) as i16);
    }
    out
}

pub fn downmix_to_mono(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|s| *s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FRAME_MS;

    #[test]
    fn downsample_produces_one_third_length_for_100ms_frame() {
        let frame_samples = (INPUT_SAMPLE_RATE * FRAME_MS / 1000) as usize;
        let input: Vec<i16> = (0..frame_samples as i16).collect();
        let out = resample_for_provider_upload(&input, 16_000);
        assert_eq!(out.len(), frame_samples / 3);
    }

    #[test]
    fn downsample_box_averages_triplets() {
        let input = vec![100, 200, 300, 400, 500, 600];
        let out = resample_for_provider_upload(&input, 16_000);
        assert_eq!(out, vec![200, 500]);
    }

    #[test]
    fn downsample_empty_returns_empty() {
        assert!(resample_for_provider_upload(&[], 16_000).is_empty());
    }

    #[test]
    fn upsample_24k_doubles_length() {
        let input = vec![0i16, 1000, -1000, 2000];
        let out = upsample_24k_to_48k(&input).unwrap();
        assert_eq!(out.len(), input.len() * 2);
    }

    #[test]
    fn resample_mono_to_rate_into_identity() {
        let input = vec![1, 2, 3, 4];
        let mut out = Vec::new();
        resample_mono_to_rate_into(&input, 48_000, 48_000, &mut out);
        assert_eq!(out, input);
    }

    #[test]
    fn resample_one_hop_44k_to_48k_length() {
        let input: Vec<i16> = (0..441).map(|i| i as i16).collect(); // 10 ms @ 44.1k
        let out = resample_mono_to_rate(&input, 44_100, 48_000);
        let ratio = 48_000.0f64 / 44_100.0;
        let expected = ((input.len() as f64) * ratio).ceil() as usize;
        assert_eq!(out.len(), expected);
    }

    #[test]
    fn downmix_to_mono_averages_stereo_frames() {
        let stereo = vec![100, 300, -100, 100];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![200, 0]);
    }

    #[test]
    fn downmix_to_mono_passes_through_mono() {
        let mono = vec![42, -42, 7];
        assert_eq!(downmix_to_mono(&mono, 1), mono);
    }
}
