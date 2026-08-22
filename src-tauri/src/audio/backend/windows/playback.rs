use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;

use super::super::super::device::{resolve_role_device, AudioRole, ResolvedDevice};
use super::super::super::playback::PlaybackHandle;
use super::super::super::playback_buffer::{
    PlaybackBufferConfig, PlaybackFillStatus, PlaybackRingBuffer,
};
use super::device::open_wasapi_device;
use super::mmcss::MmcssGuard;
use super::wasapi_util::{init_com, mono_i16_to_device_bytes_into, open_stream, wait_for_audio};

pub fn start_playback(
    resolved: ResolvedDevice,
    mut rx: mpsc::Receiver<Vec<i16>>,
    buffer_config: PlaybackBufferConfig,
) -> Result<PlaybackHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let device = resolved.clone();

    let thread = std::thread::Builder::new()
        .name("wasapi-playback".into())
        .spawn(move || {
            if let Err(e) = playback_worker(&device, stop_flag, &mut rx, buffer_config) {
                tracing::error!(
                    "playback thread error for {} ({}): {e:#}",
                    device.name,
                    device.id
                );
            }
        })?;

    Ok(PlaybackHandle::new(stop, thread))
}

pub fn start_playback_for_role(
    role: AudioRole,
    config: &crate::config::AppConfig,
    devices: &[super::super::super::device::AudioDeviceInfo],
    rx: mpsc::Receiver<Vec<i16>>,
) -> Result<PlaybackHandle> {
    let resolved = resolve_role_device(role, config, devices).map_err(|e| anyhow::anyhow!(e))?;
    start_playback(resolved, rx, PlaybackBufferConfig::default())
}

fn playback_worker(
    resolved: &ResolvedDevice,
    stop: Arc<AtomicBool>,
    rx: &mut mpsc::Receiver<Vec<i16>>,
    buffer_config: PlaybackBufferConfig,
) -> Result<()> {
    let _mmcss = MmcssGuard::enter();
    init_com()?;

    const MAX_ATTEMPTS: u32 = 6;
    for attempt in 0..MAX_ATTEMPTS {
        if stop.load(Ordering::SeqCst) {
            return Ok(());
        }
        if attempt > 0 {
            tracing::warn!(
                "retrying playback open for {} ({}) — attempt {}",
                resolved.name,
                resolved.id,
                attempt + 1
            );
            std::thread::sleep(Duration::from_millis(120 * attempt as u64));
        }

        match run_playback_session(resolved, &stop, rx, buffer_config.clone()) {
            Ok(()) => return Ok(()),
            Err(e) if attempt + 1 < MAX_ATTEMPTS => {
                tracing::warn!(
                    "playback open failed for {} ({}): {e:#}",
                    resolved.name,
                    resolved.id
                );
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

fn run_playback_session(
    resolved: &ResolvedDevice,
    stop: &Arc<AtomicBool>,
    rx: &mut mpsc::Receiver<Vec<i16>>,
    buffer_config: PlaybackBufferConfig,
) -> Result<()> {
    use wasapi::Direction;

    let wasapi_device = open_wasapi_device(resolved)?;
    let label = format!("{} ({})", resolved.name, resolved.id);
    let (audio_client, stream) = open_stream(&wasapi_device, &Direction::Render, &label)?;
    let sample_type = stream.format.get_subformat()?;
    let render_client = audio_client.get_audiorenderclient()?;
    let mut sample_queue: VecDeque<u8> = VecDeque::new();
    let mut scratch_device_bytes = Vec::with_capacity(4096);
    let mut ring = PlaybackRingBuffer::new(buffer_config, stream.sample_rate);

    audio_client.start_stream()?;
    tracing::info!(
        "playback opened: {} ({} Hz, {} ch, ring buffer)",
        label,
        stream.sample_rate,
        stream.channels
    );

    while !stop.load(Ordering::SeqCst) {
        let buffer_frame_count = audio_client.get_available_space_in_frames()?;
        let needed_bytes = stream.blockalign as usize * buffer_frame_count as usize;
        let blockalign = stream.blockalign as usize;

        while sample_queue.len() < needed_bytes {
            let frames_still_needed = (needed_bytes - sample_queue.len()).div_ceil(blockalign);
            let frames_still_needed = frames_still_needed.max(1);

            match ring.ensure_device_samples_blocking(frames_still_needed, rx) {
                PlaybackFillStatus::Disconnected if ring.is_empty() => {
                    audio_client.stop_stream()?;
                    return Ok(());
                }
                PlaybackFillStatus::Disconnected | PlaybackFillStatus::Ok => {}
            }

            let device_samples = ring.pop_device_samples(frames_still_needed);
            mono_i16_to_device_bytes_into(
                &device_samples,
                stream.channels,
                &sample_type,
                &mut scratch_device_bytes,
            );
            sample_queue.extend(scratch_device_bytes.iter().copied());
        }

        render_client.write_to_device_from_deque(
            buffer_frame_count as usize,
            &mut sample_queue,
            None,
        )?;

        wait_for_audio(&stream.wait_mode);
    }

    audio_client.stop_stream()?;
    Ok(())
}
