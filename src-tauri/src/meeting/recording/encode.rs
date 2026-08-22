//! Ogg Opus encode/decode for meeting audio chunks (RFC 7845).

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use ogg::reading::PacketReader;
use ogg::writing::{PacketWriteEndInfo, PacketWriter};
use opus::{Application, Bitrate, Channels, Decoder, Encoder};

pub const RECORD_SAMPLE_RATE: u32 = 16_000;
const FRAME_SAMPLES: usize = 320; // 20 ms @ 16 kHz
/// RFC 7845 granule is always in 48 kHz units (20 ms → 960).
const GRANULE_PER_FRAME_48K: u64 = 960;
const OGG_MAGIC: &[u8; 4] = b"OggS";
const CACHE_MAX_ENTRIES: usize = 6;

/// Encode mono i16 @ `RECORD_SAMPLE_RATE` to a standard Ogg Opus `.opus` file.
pub fn encode_pcm_i16_to_file(pcm: &[i16], path: &Path) -> Result<()> {
    if pcm.is_empty() {
        bail!("empty pcm");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create chunk parent dir")?;
    }

    let mut encoder = Encoder::new(RECORD_SAMPLE_RATE, Channels::Mono, Application::Voip)
        .context("create opus encoder")?;
    encoder
        .set_bitrate(Bitrate::Bits(24_000))
        .context("set opus bitrate")?;

    let file = File::create(path).context("create ogg opus file")?;
    let mut writer = PacketWriter::new(file);
    let serial = 0x4D54_5231u32; // "MTR1"

    writer
        .write_packet(
            Cow::Owned(opus_head_packet(RECORD_SAMPLE_RATE)),
            serial,
            PacketWriteEndInfo::EndPage,
            0,
        )
        .context("write OpusHead")?;
    writer
        .write_packet(
            Cow::Owned(opus_tags_packet()),
            serial,
            PacketWriteEndInfo::EndPage,
            0,
        )
        .context("write OpusTags")?;

    let mut offset = 0usize;
    let mut packet_buf = vec![0u8; 4000];
    let mut frame = vec![0i16; FRAME_SAMPLES];
    let mut granule: u64 = 0;
    let total_frames = pcm.len().div_ceil(FRAME_SAMPLES);

    for frame_idx in 0..total_frames {
        let end = (offset + FRAME_SAMPLES).min(pcm.len());
        frame.fill(0);
        let copy_len = end - offset;
        frame[..copy_len].copy_from_slice(&pcm[offset..end]);
        let len = encoder
            .encode(&frame, &mut packet_buf)
            .context("opus encode frame")?;
        granule = granule.saturating_add(GRANULE_PER_FRAME_48K);
        let is_last = frame_idx + 1 == total_frames;
        let end_info = if is_last {
            PacketWriteEndInfo::EndStream
        } else {
            PacketWriteEndInfo::NormalPacket
        };
        writer
            .write_packet(
                Cow::Owned(packet_buf[..len].to_vec()),
                serial,
                end_info,
                granule,
            )
            .context("write opus audio packet")?;
        offset = end;
    }

    invalidate_decode_cache(path);
    Ok(())
}

/// Decode meeting audio file to mono f32 PCM (Ogg Opus only).
pub fn decode_pcm_f32(path: &Path) -> Result<(u32, Vec<f32>)> {
    if let Some(hit) = cache_get(path) {
        return Ok((hit.0, hit.1.clone()));
    }
    let (sr, samples) = decode_pcm_f32_uncached(path)?;
    cache_put(path, sr, samples.clone());
    Ok((sr, samples))
}

/// Decode without touching the cache (tests / refresh).
pub fn decode_pcm_f32_uncached(path: &Path) -> Result<(u32, Vec<f32>)> {
    let mut probe = [0u8; 4];
    {
        let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
        f.read_exact(&mut probe).context("probe magic")?;
    }
    if &probe != OGG_MAGIC {
        bail!("unsupported meeting audio format (expected Ogg Opus)");
    }
    decode_ogg_opus(path)
}

fn opus_head_packet(sample_rate: u32) -> Vec<u8> {
    // RFC 7845 §5.1 Identification Header
    let mut p = Vec::with_capacity(19);
    p.extend_from_slice(b"OpusHead");
    p.push(1); // version
    p.push(1); // channel count
    p.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
    p.extend_from_slice(&sample_rate.to_le_bytes());
    p.extend_from_slice(&0i16.to_le_bytes()); // output gain
    p.push(0); // channel mapping family 0
    p
}

fn opus_tags_packet() -> Vec<u8> {
    let vendor = b"Meetral";
    let mut p = Vec::with_capacity(16 + vendor.len());
    p.extend_from_slice(b"OpusTags");
    p.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    p.extend_from_slice(vendor);
    p.extend_from_slice(&0u32.to_le_bytes()); // user comment list count
    p
}

fn decode_ogg_opus(path: &Path) -> Result<(u32, Vec<f32>)> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut reader = PacketReader::new(file);
    let mut sample_rate = RECORD_SAMPLE_RATE;
    let mut saw_head = false;
    let mut saw_tags = false;
    let mut decoder: Option<Decoder> = None;
    let mut out_i16 = Vec::new();
    let mut pcm_frame = vec![0i16; FRAME_SAMPLES * 2];

    while let Some(packet) = reader.read_packet().context("read ogg packet")? {
        let data = packet.data;
        if !saw_head {
            if data.len() >= 8 && &data[..8] == b"OpusHead" {
                if data.len() >= 12 {
                    sample_rate = u32::from_le_bytes(data[12..16].try_into().unwrap_or([0; 4]));
                    if sample_rate == 0 {
                        sample_rate = RECORD_SAMPLE_RATE;
                    }
                }
                decoder =
                    Some(Decoder::new(sample_rate, Channels::Mono).context("create opus decoder")?);
                saw_head = true;
                continue;
            }
            bail!("missing OpusHead in ogg stream");
        }
        if !saw_tags {
            // Second packet must be OpusTags (skip contents).
            saw_tags = true;
            continue;
        }
        let dec = decoder.as_mut().context("decoder missing")?;
        let n = dec
            .decode(&data, &mut pcm_frame, false)
            .context("opus decode")?;
        out_i16.extend_from_slice(&pcm_frame[..n]);
    }

    if !saw_head {
        bail!("empty or invalid ogg opus file");
    }

    let samples: Vec<f32> = out_i16.iter().map(|&s| s as f32 / 32768.0).collect();
    Ok((sample_rate, samples))
}

// --- Decode cache -----------------------------------------------------------

struct DecodeCache {
    map: HashMap<String, Arc<(u32, Vec<f32>)>>,
    order: VecDeque<String>,
}

impl DecodeCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

static DECODE_CACHE: Mutex<Option<DecodeCache>> = Mutex::new(None);

fn with_cache<R>(f: impl FnOnce(&mut DecodeCache) -> R) -> Option<R> {
    let mut guard = DECODE_CACHE.lock().ok()?;
    if guard.is_none() {
        *guard = Some(DecodeCache::new());
    }
    Some(f(guard.as_mut().unwrap()))
}

fn cache_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn cache_get(path: &Path) -> Option<Arc<(u32, Vec<f32>)>> {
    let key = cache_key(path);
    with_cache(|c| c.map.get(&key).cloned()).flatten()
}

fn cache_put(path: &Path, sr: u32, samples: Vec<f32>) {
    let key = cache_key(path);
    let _ = with_cache(|c| {
        if c.map.contains_key(&key) {
            return;
        }
        while c.order.len() >= CACHE_MAX_ENTRIES {
            if let Some(old) = c.order.pop_front() {
                c.map.remove(&old);
            } else {
                break;
            }
        }
        c.order.push_back(key.clone());
        c.map.insert(key, Arc::new((sr, samples)));
    });
}

fn invalidate_decode_cache(path: &Path) {
    let key = cache_key(path);
    let _ = with_cache(|c| {
        c.map.remove(&key);
        c.order.retain(|k| k != &key);
    });
}

/// Clear decode cache (tests).
pub fn clear_decode_cache() {
    let _ = with_cache(|c| {
        c.map.clear();
        c.order.clear();
    });
}

/// True if path bytes look like Ogg (for diagnostics).
pub fn is_ogg_file(path: &Path) -> bool {
    let mut probe = [0u8; 4];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut probe))
        .map(|_| &probe == OGG_MAGIC)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ogg_roundtrip_tone() {
        clear_decode_cache();
        let n = FRAME_SAMPLES * 5;
        let pcm: Vec<i16> = (0..n)
            .map(|i| if i % 2 == 0 { 12_000 } else { -12_000 })
            .collect();
        let path =
            std::env::temp_dir().join(format!("meetral_ogg_test_{}.opus", std::process::id()));
        encode_pcm_i16_to_file(&pcm, &path).unwrap();
        assert!(is_ogg_file(&path));
        let (sr, samples) = decode_pcm_f32_uncached(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(sr, RECORD_SAMPLE_RATE);
        assert!(samples.len() >= FRAME_SAMPLES * 4);
        let peak = samples.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        assert!(peak > 0.2, "peak={peak}");
    }

    #[test]
    fn decode_cache_returns_same_energy() {
        clear_decode_cache();
        let pcm = vec![8_000i16; FRAME_SAMPLES * 3];
        let path =
            std::env::temp_dir().join(format!("meetral_ogg_cache_{}.opus", std::process::id()));
        encode_pcm_i16_to_file(&pcm, &path).unwrap();
        let (sr1, a) = decode_pcm_f32(&path).unwrap();
        let (sr2, b) = decode_pcm_f32(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(sr1, sr2);
        assert_eq!(a.len(), b.len());
        assert!((a[10] - b[10]).abs() < 1e-6);
    }

    #[test]
    fn ogg_head_in_memory_roundtrip_probe() {
        use std::io::Cursor;
        let head = opus_head_packet(16_000);
        assert_eq!(&head[..8], b"OpusHead");
        let mut cur = Cursor::new(Vec::new());
        {
            let mut w = PacketWriter::new(&mut cur);
            w.write_packet(Cow::Owned(head), 1, PacketWriteEndInfo::EndPage, 0)
                .unwrap();
            w.write_packet(
                Cow::Owned(opus_tags_packet()),
                1,
                PacketWriteEndInfo::EndPage,
                0,
            )
            .unwrap();
        }
        let bytes = cur.into_inner();
        assert_eq!(&bytes[..4], OGG_MAGIC);
    }
}
