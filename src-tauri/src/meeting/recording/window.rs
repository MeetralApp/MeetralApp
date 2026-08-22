//! Decode a time window of meeting recording chunks for playback preview.

use super::{decode_pcm_f32, RECORD_SAMPLE_RATE};
use crate::meeting::store::AudioChunkRow;

#[derive(Debug, Clone)]
pub struct DecodedAudioWindow {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

pub fn decode_window_sync(
    chunks: &[AudioChunkRow],
    source: &str,
    start_ms: i64,
    duration_ms: i64,
) -> anyhow::Result<DecodedAudioWindow> {
    let end_ms = start_ms + duration_ms;
    let want_in = source == "room" || source == "meeting";
    let want_out = source == "room" || source == "you";

    let inbound = if want_in {
        decode_direction_window(chunks, "inbound", start_ms, end_ms)?
    } else {
        Vec::new()
    };
    let outbound = if want_out {
        decode_direction_window(chunks, "outbound", start_ms, end_ms)?
    } else {
        Vec::new()
    };

    let samples = match source {
        "you" => outbound,
        "meeting" => inbound,
        _ => mix_room(&inbound, &outbound),
    };

    Ok(DecodedAudioWindow {
        sample_rate: RECORD_SAMPLE_RATE,
        samples,
    })
}

pub fn decode_direction_window(
    chunks: &[AudioChunkRow],
    direction: &str,
    start_ms: i64,
    end_ms: i64,
) -> anyhow::Result<Vec<f32>> {
    let mut out = Vec::new();
    let target_len =
        ((end_ms - start_ms).max(0) as u64 * RECORD_SAMPLE_RATE as u64 / 1000) as usize;
    out.resize(target_len, 0.0);

    for chunk in chunks.iter().filter(|c| c.direction == direction) {
        let c_start = chunk.started_at_ms;
        let c_end = c_start + chunk.duration_ms;
        if c_end <= start_ms || c_start >= end_ms {
            continue;
        }
        let (sr, pcm) = match decode_pcm_f32(std::path::Path::new(&chunk.file_path)) {
            Ok(decoded) => decoded,
            Err(err) => {
                tracing::warn!(
                    path = %chunk.file_path,
                    error = %err,
                    "skip missing/unreadable meeting audio chunk"
                );
                continue;
            }
        };
        if sr == 0 || pcm.is_empty() {
            continue;
        }
        // Contiguous sample copy — do NOT map via ms-rounded indices (that
        // keeps only 1 of every 16 samples at 16 kHz and leaves silence holes).
        let overlap_start = start_ms.max(c_start);
        let overlap_end = end_ms.min(c_end);
        let src_start = (((overlap_start - c_start).max(0) as u64) * sr as u64 / 1000) as usize;
        let src_end = (((overlap_end - c_start).max(0) as u64) * sr as u64 / 1000) as usize;
        let src_end = src_end.min(pcm.len());
        if src_start >= src_end {
            continue;
        }
        let dst_start = (((overlap_start - start_ms).max(0) as u64) * RECORD_SAMPLE_RATE as u64
            / 1000) as usize;
        let copy_len = (src_end - src_start).min(out.len().saturating_sub(dst_start));
        if copy_len == 0 {
            continue;
        }
        // If chunk sample rate differs from playback rate, nearest-neighbor
        // (recordings are always 16 kHz today).
        if sr == RECORD_SAMPLE_RATE {
            out[dst_start..dst_start + copy_len]
                .copy_from_slice(&pcm[src_start..src_start + copy_len]);
        } else {
            for i in 0..copy_len {
                let src_i = src_start + i * sr as usize / RECORD_SAMPLE_RATE as usize;
                if src_i < pcm.len() && dst_start + i < out.len() {
                    out[dst_start + i] = pcm[src_i];
                }
            }
        }
    }
    Ok(out)
}

pub fn mix_room(inbound: &[f32], outbound: &[f32]) -> Vec<f32> {
    let len = inbound.len().max(outbound.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let a = inbound.get(i).copied().unwrap_or(0.0);
        let b = outbound.get(i).copied().unwrap_or(0.0);
        // Light duck of inbound when outbound is loud
        let duck = if b.abs() > 0.08 { 0.55 } else { 1.0 };
        let mixed = (a * duck + b).clamp(-1.0, 1.0);
        out.push(mixed);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meeting::recording::{encode_pcm_i16_to_file, is_ogg_file, RECORD_SAMPLE_RATE};
    use crate::meeting::store::AudioChunkRow;

    #[test]
    fn decode_window_preserves_contiguous_pcm_energy() {
        let n = RECORD_SAMPLE_RATE as usize / 5;
        let pcm: Vec<i16> = (0..n)
            .map(|i| if i % 2 == 0 { 20_000 } else { -20_000 })
            .collect();
        let path =
            std::env::temp_dir().join(format!("meetral_win_test_{}.opus", std::process::id()));
        encode_pcm_i16_to_file(&pcm, &path).unwrap();

        let chunks = vec![AudioChunkRow {
            id: "c1".into(),
            meeting_id: "m1".into(),
            direction: "inbound".into(),
            sequence: 0,
            file_path: path.to_string_lossy().to_string(),
            duration_ms: 200,
            sample_rate: RECORD_SAMPLE_RATE as i64,
            started_at_ms: 0,
            byte_size: 0,
        }];
        let window = decode_direction_window(&chunks, "inbound", 0, 200).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(window.len(), RECORD_SAMPLE_RATE as usize / 5);
        let peak = window.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        let nonzero = window.iter().filter(|s| s.abs() > 0.05).count();
        assert!(peak > 0.4, "peak={peak}");
        assert!(
            nonzero as f32 / window.len() as f32 > 0.8,
            "nonzero ratio too low: {nonzero}/{}",
            window.len()
        );
    }

    #[test]
    fn decode_inbound_fixture_if_env_path_present() {
        let Some(raw) = std::env::var_os("MEETRAL_DECODE_FIXTURE") else {
            return;
        };
        let path = std::path::PathBuf::from(raw);
        if !path.exists() || !is_ogg_file(&path) {
            return;
        }
        let chunks = vec![AudioChunkRow {
            id: "c1".into(),
            meeting_id: "fixture".into(),
            direction: "inbound".into(),
            sequence: 0,
            file_path: path.to_string_lossy().to_string(),
            duration_ms: 30_000,
            sample_rate: RECORD_SAMPLE_RATE as i64,
            started_at_ms: 1400,
            byte_size: 0,
        }];
        let window = decode_direction_window(&chunks, "inbound", 1400, 2900).unwrap();
        let peak = window.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        let nonzero = window.iter().filter(|s| s.abs() > 1e-3).count();
        assert!(peak > 0.05, "peak={peak}");
        assert!(
            nonzero as f32 / window.len() as f32 > 0.5,
            "nonzero ratio {nonzero}/{}",
            window.len()
        );
    }
}
