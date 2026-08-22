// Single-threaded capture → render passthrough for Direct mode on Windows.
//
// One thread per relay direction. Capture and render are interleaved so playback
// is fed at device period rate while the thread only sleeps when both paths are idle.

use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Result;

use super::super::super::capture::{
    AudioFaultEvent, AudioFaultSender, CaptureHandle, CaptureHeartbeat,
};
use super::super::super::device::ResolvedDevice;
use super::super::super::resampler::resample_mono_to_rate_into;
use super::device::open_wasapi_device;
use super::mmcss::MmcssGuard;
use super::wasapi_util::{
    bytes_to_mono_i16_into, init_com, mono_i16_to_device_bytes_into, open_stream, wait_for_audio,
    wait_for_audio_paced, StreamSetup,
};
use crate::audio::gate_pcm_in_place;
use crate::config::INPUT_SAMPLE_RATE;

/// Smaller than the translate pipeline frame (100 ms) for smoother local passthrough.
const DIRECT_FRAME_MS: u32 = 20;
/// Max queued render audio before dropping oldest frames (~80 ms at 20 ms/frame).
const MAX_RENDER_PENDING_FRAMES: usize = 4;

pub fn start_direct_passthrough(
    capture_device: ResolvedDevice,
    playback_device: ResolvedDevice,
    mute_gate: Arc<AtomicBool>,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<AudioFaultSender>,
    thread_name: &'static str,
) -> Result<CaptureHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let capture_id = capture_device.id.clone();
    let capture_label = capture_device.name.clone();
    let fault_device_id = capture_device.id.clone();
    let fault_device_name = capture_device.name.clone();
    let heartbeat_worker = heartbeat.clone();

    let thread = std::thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            if let Err(e) = passthrough_worker(
                &capture_device,
                &playback_device,
                stop_flag,
                &mute_gate,
                &heartbeat_worker,
            ) {
                tracing::error!(
                    "direct passthrough error {} → {}: {e:#}",
                    capture_label,
                    playback_device.name
                );
                if let Some(tx) = fault_tx {
                    crate::runtime::control_channel::try_send_control(
                        &tx,
                        AudioFaultEvent {
                            device_id: fault_device_id,
                            device_name: fault_device_name,
                            reason: format!("{e:#}"),
                        },
                        "audio-fault",
                    );
                }
            }
        })?;

    Ok(CaptureHandle::new(stop, thread, capture_id, heartbeat))
}

fn passthrough_worker(
    capture_resolved: &ResolvedDevice,
    playback_resolved: &ResolvedDevice,
    stop: Arc<AtomicBool>,
    mute_gate: &AtomicBool,
    heartbeat: &CaptureHeartbeat,
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
                "retrying direct passthrough for {} → {} — attempt {}",
                capture_resolved.name,
                playback_resolved.name,
                attempt + 1
            );
            std::thread::sleep(Duration::from_millis(120 * attempt as u64));
        }

        match run_passthrough_session(
            capture_resolved,
            playback_resolved,
            &stop,
            mute_gate,
            heartbeat,
        ) {
            Ok(()) => return Ok(()),
            Err(e) if attempt + 1 < MAX_ATTEMPTS => {
                tracing::warn!(
                    "direct passthrough open failed for {} → {}: {e:#}",
                    capture_resolved.name,
                    playback_resolved.name
                );
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

fn run_passthrough_session(
    capture_resolved: &ResolvedDevice,
    playback_resolved: &ResolvedDevice,
    stop: &Arc<AtomicBool>,
    mute_gate: &AtomicBool,
    heartbeat: &CaptureHeartbeat,
) -> Result<()> {
    use wasapi::Direction;

    let capture_wasapi = open_wasapi_device(capture_resolved)?;
    let playback_wasapi = open_wasapi_device(playback_resolved)?;

    let capture_label = format!("{} ({})", capture_resolved.name, capture_resolved.id);
    let playback_label = format!("{} ({})", playback_resolved.name, playback_resolved.id);

    let (capture_client, capture_stream) =
        open_stream(&capture_wasapi, &Direction::Capture, &capture_label)?;
    let capture_sample_type = capture_stream.format.get_subformat()?;
    let capture_io = capture_client.get_audiocaptureclient()?;

    let (render_client, render_stream) =
        open_stream(&playback_wasapi, &Direction::Render, &playback_label)?;
    let render_sample_type = render_stream.format.get_subformat()?;
    let render_io = render_client.get_audiorenderclient()?;

    let frame_samples = (INPUT_SAMPLE_RATE * DIRECT_FRAME_MS / 1000) as usize;
    let capture_chunk_bytes = capture_stream.blockalign as usize * frame_samples;
    let render_frame_samples =
        (render_stream.sample_rate as u64 * DIRECT_FRAME_MS as u64 / 1000).max(1) as usize;
    let render_frame_bytes = render_stream.blockalign as usize * render_frame_samples;
    let max_render_pending_bytes = render_frame_bytes * MAX_RENDER_PENDING_FRAMES;
    let mut capture_queue: VecDeque<u8> = VecDeque::with_capacity(capture_chunk_bytes * 2);
    let mut render_pending: VecDeque<u8> =
        VecDeque::with_capacity(max_render_pending_bytes + render_frame_bytes);
    let mut scratch = PassthroughScratch {
        bytes: Vec::with_capacity(capture_chunk_bytes),
        pcm: Vec::with_capacity(frame_samples),
        resample: Vec::with_capacity(frame_samples.max(render_frame_samples)),
        device_bytes: Vec::with_capacity(render_frame_bytes),
    };

    capture_client.start_stream()?;
    render_client.start_stream()?;

    tracing::info!(
        "direct passthrough: {} → {} ({} ms frames, capture {} Hz, render {} Hz)",
        capture_label,
        playback_label,
        DIRECT_FRAME_MS,
        capture_stream.sample_rate,
        render_stream.sample_rate
    );

    let idle_wait = Duration::from_millis(DIRECT_FRAME_MS as u64);

    while !stop.load(Ordering::SeqCst) {
        let mut did_work = false;

        if drain_render_pending(
            &render_client,
            &render_stream,
            &render_io,
            &mut render_pending,
        )? {
            did_work = true;
        }

        if capture_queue.len() >= capture_chunk_bytes {
            enqueue_capture_frame(
                &mut capture_queue,
                capture_chunk_bytes,
                &capture_stream,
                &capture_sample_type,
                &render_stream,
                &render_sample_type,
                mute_gate,
                heartbeat,
                &mut render_pending,
                render_frame_bytes,
                max_render_pending_bytes,
                &mut scratch,
            );
            did_work = true;
        } else {
            let queue_before = capture_queue.len();
            capture_io.read_from_device_to_deque(&mut capture_queue)?;
            if capture_queue.len() > queue_before {
                did_work = true;
            }
        }

        if did_work {
            if !render_pending.is_empty() {
                wait_for_audio(&render_stream.wait_mode);
            } else if capture_queue.len() < capture_chunk_bytes {
                wait_for_audio(&capture_stream.wait_mode);
            }
        } else if !render_pending.is_empty() {
            wait_for_audio(&render_stream.wait_mode);
        } else {
            wait_for_audio_paced(&capture_stream.wait_mode, idle_wait);
        }
    }

    render_client.stop_stream()?;
    capture_client.stop_stream()?;
    Ok(())
}

struct PassthroughScratch {
    bytes: Vec<u8>,
    pcm: Vec<i16>,
    resample: Vec<i16>,
    device_bytes: Vec<u8>,
}

fn enqueue_capture_frame(
    capture_queue: &mut VecDeque<u8>,
    capture_chunk_bytes: usize,
    capture_stream: &StreamSetup,
    capture_sample_type: &wasapi::SampleType,
    render_stream: &StreamSetup,
    render_sample_type: &wasapi::SampleType,
    mute_gate: &AtomicBool,
    heartbeat: &CaptureHeartbeat,
    render_pending: &mut VecDeque<u8>,
    render_frame_bytes: usize,
    max_render_pending_bytes: usize,
    scratch: &mut PassthroughScratch,
) {
    scratch.bytes.clear();
    scratch
        .bytes
        .extend(capture_queue.drain(..capture_chunk_bytes));
    bytes_to_mono_i16_into(
        &scratch.bytes,
        capture_stream.channels as usize,
        capture_sample_type,
        &mut scratch.pcm,
    );
    if capture_stream.sample_rate != INPUT_SAMPLE_RATE {
        resample_mono_to_rate_into(
            &scratch.pcm,
            capture_stream.sample_rate,
            INPUT_SAMPLE_RATE,
            &mut scratch.resample,
        );
        std::mem::swap(&mut scratch.pcm, &mut scratch.resample);
    }
    gate_pcm_in_place(mute_gate, &mut scratch.pcm);
    heartbeat.touch();

    let playback = if render_stream.sample_rate == INPUT_SAMPLE_RATE {
        &scratch.pcm[..]
    } else {
        resample_mono_to_rate_into(
            &scratch.pcm,
            INPUT_SAMPLE_RATE,
            render_stream.sample_rate,
            &mut scratch.resample,
        );
        &scratch.resample[..]
    };
    mono_i16_to_device_bytes_into(
        playback,
        render_stream.channels,
        render_sample_type,
        &mut scratch.device_bytes,
    );
    render_pending.extend(scratch.device_bytes.iter().copied());
    trim_render_backlog(render_pending, max_render_pending_bytes, render_frame_bytes);
}

fn trim_render_backlog(pending: &mut VecDeque<u8>, max_bytes: usize, frame_bytes: usize) {
    if frame_bytes == 0 {
        return;
    }
    let mut dropped_frames = 0u32;
    while pending.len() > max_bytes {
        let drop = frame_bytes.min(pending.len());
        pending.drain(..drop);
        dropped_frames += 1;
    }
    if dropped_frames > 0 {
        tracing::debug!(
            "direct passthrough: dropped {dropped_frames} stale render frame(s) (cap {max_bytes} bytes)"
        );
    }
}

fn drain_render_pending(
    audio_client: &wasapi::AudioClient,
    stream: &StreamSetup,
    render_client: &wasapi::AudioRenderClient,
    pending: &mut VecDeque<u8>,
) -> Result<bool> {
    if pending.is_empty() {
        return Ok(false);
    }

    let buffer_frames = audio_client.get_available_space_in_frames()?;
    if buffer_frames == 0 {
        return Ok(false);
    }

    let blockalign = stream.blockalign as usize;
    let needed_bytes = blockalign * buffer_frames as usize;
    while pending.len() < needed_bytes {
        pending.push_back(0);
    }
    render_client.write_to_device_from_deque(buffer_frames as usize, pending, None)?;
    Ok(true)
}
