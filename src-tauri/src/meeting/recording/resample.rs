//! 48 kHz → 16 kHz resample via rubato.

use anyhow::{Context, Result};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};

use super::encode::RECORD_SAMPLE_RATE;
use crate::config::INPUT_SAMPLE_RATE;

const IN_CHUNK: usize = 1024;

/// Downsample mono i16 from capture rate (48 kHz) to record rate (16 kHz).
pub fn resample_48k_to_16k(pcm: &[i16]) -> Result<Vec<i16>> {
    if pcm.is_empty() {
        return Ok(Vec::new());
    }
    debug_assert_eq!(INPUT_SAMPLE_RATE, 48_000);
    debug_assert_eq!(RECORD_SAMPLE_RATE, 16_000);

    let ratio = RECORD_SAMPLE_RATE as f64 / INPUT_SAMPLE_RATE as f64;
    let mut resampler = FastFixedIn::<f32>::new(ratio, 1.0, PolynomialDegree::Linear, IN_CHUNK, 1)
        .context("create rubato resampler")?;

    let mut input_f: Vec<f32> = pcm.iter().map(|&s| s as f32 / 32768.0).collect();
    // Pad to whole input chunks so rubato can flush predictably.
    let rem = input_f.len() % IN_CHUNK;
    if rem != 0 {
        input_f.resize(input_f.len() + (IN_CHUNK - rem), 0.0);
    }

    let mut out = Vec::with_capacity(input_f.len() / 3 + 64);
    let mut pos = 0;
    while pos + IN_CHUNK <= input_f.len() {
        let wave = [&input_f[pos..pos + IN_CHUNK]];
        let waves = resampler.process(&wave, None).context("rubato process")?;
        if let Some(ch0) = waves.first() {
            for &s in ch0 {
                let clamped = s.clamp(-1.0, 1.0);
                out.push((clamped * 32767.0) as i16);
            }
        }
        pos += IN_CHUNK;
    }

    // Trim padding-induced tail back to nominal length.
    let expected = pcm.len() / 3;
    if out.len() > expected {
        out.truncate(expected);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_ratio_approx_one_third() {
        let input: Vec<i16> = (0..4800).map(|i| ((i % 200) as i16 - 100) * 100).collect();
        let out = resample_48k_to_16k(&input).unwrap();
        assert_eq!(out.len(), 1600);
        let peak = out.iter().map(|s| s.abs()).max().unwrap_or(0);
        assert!(peak > 100, "peak too low after resample: {peak}");
    }
}
