use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;

use super::super::super::capture::{
    AudioFaultEvent, AudioFaultSender, CaptureHandle, CaptureHeartbeat, CaptureSender,
};
use super::super::super::device::{resolve_role_device, AudioRole, ResolvedDevice};
use super::super::super::resampler::resample_mono_to_rate_into;
use super::device::open_wasapi_device;
use super::mmcss::MmcssGuard;
use super::wasapi_util::{bytes_to_mono_i16_into, init_com, open_stream, wait_for_audio};
use crate::config::{AppConfig, FRAME_MS, INPUT_SAMPLE_RATE};

pub fn start_user_mic_capture(
    config: &AppConfig,
    devices: &[super::super::super::device::AudioDeviceInfo],
    tx: CaptureSender,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<AudioFaultSender>,
) -> Result<CaptureHandle> {
    let resolved =
        resolve_role_device(AudioRole::UserMic, config, devices).map_err(|e| anyhow::anyhow!(e))?;
    start_capture_thread(resolved, tx, heartbeat, fault_tx)
}

pub fn start_meeting_capture_for_config(
    config: &AppConfig,
    devices: &[super::super::super::device::AudioDeviceInfo],
    tx: CaptureSender,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<AudioFaultSender>,
) -> Result<CaptureHandle> {
    let resolved = resolve_role_device(AudioRole::MeetingCapture, config, devices)
        .map_err(|e| anyhow::anyhow!(e))?;
    start_capture_thread(resolved, tx, heartbeat, fault_tx)
}

fn start_capture_thread(
    resolved: ResolvedDevice,
    tx: CaptureSender,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<AudioFaultSender>,
) -> Result<CaptureHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let device = resolved.clone();
    let device_id = resolved.id.clone();
    let heartbeat_worker = heartbeat.clone();

    let fault_device_id = device.id.clone();
    let fault_device_name = device.name.clone();
    let thread = std::thread::Builder::new()
        .name("wasapi-capture".into())
        .spawn(move || {
            if let Err(e) = capture_worker(&device, stop_flag, tx, heartbeat_worker) {
                tracing::error!(
                    "capture thread error for {} ({}): {e:#}",
                    device.name,
                    device.id
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

    Ok(CaptureHandle::new(stop, thread, device_id, heartbeat))
}

fn capture_worker(
    resolved: &ResolvedDevice,
    stop: Arc<AtomicBool>,
    tx: CaptureSender,
    heartbeat: CaptureHeartbeat,
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
                "retrying capture open for {} ({}) — attempt {}",
                resolved.name,
                resolved.id,
                attempt + 1
            );
            std::thread::sleep(Duration::from_millis(120 * attempt as u64));
        }

        match run_capture_session(resolved, &stop, &tx, &heartbeat) {
            Ok(()) => return Ok(()),
            Err(e) if attempt + 1 < MAX_ATTEMPTS => {
                tracing::warn!(
                    "capture open failed for {} ({}): {e:#}",
                    resolved.name,
                    resolved.id
                );
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

fn run_capture_session(
    resolved: &ResolvedDevice,
    stop: &Arc<AtomicBool>,
    tx: &CaptureSender,
    heartbeat: &CaptureHeartbeat,
) -> Result<()> {
    use wasapi::Direction;

    let wasapi_device = open_wasapi_device(resolved)?;
    let label = format!("{} ({})", resolved.name, resolved.id);
    let (audio_client, stream) = open_stream(&wasapi_device, &Direction::Capture, &label)?;
    let sample_type = stream.format.get_subformat()?;
    let capture_client = audio_client.get_audiocaptureclient()?;

    let frame_samples = (INPUT_SAMPLE_RATE * FRAME_MS / 1000) as usize;
    let chunk_bytes = stream.blockalign as usize * frame_samples;
    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(chunk_bytes * 4);
    let mut scratch_bytes = Vec::with_capacity(chunk_bytes);
    let mut scratch_pcm = Vec::with_capacity(frame_samples);
    let mut scratch_resample = Vec::with_capacity(frame_samples);

    audio_client.start_stream()?;
    tracing::info!(
        "capture opened: {} ({} Hz, {} ch)",
        label,
        stream.sample_rate,
        stream.channels
    );

    'capture: while !stop.load(Ordering::SeqCst) {
        while sample_queue.len() >= chunk_bytes {
            scratch_bytes.clear();
            scratch_bytes.extend(sample_queue.drain(..chunk_bytes));
            bytes_to_mono_i16_into(
                &scratch_bytes,
                stream.channels as usize,
                &sample_type,
                &mut scratch_pcm,
            );
            let pcm_to_send = if stream.sample_rate != INPUT_SAMPLE_RATE {
                resample_mono_to_rate_into(
                    &scratch_pcm,
                    stream.sample_rate,
                    INPUT_SAMPLE_RATE,
                    &mut scratch_resample,
                );
                std::mem::take(&mut scratch_resample)
            } else {
                std::mem::take(&mut scratch_pcm)
            };
            match tx.try_send(pcm_to_send) {
                Ok(()) => heartbeat.touch(),
                Err(mpsc::error::TrySendError::Full(_)) => heartbeat.touch(),
                Err(mpsc::error::TrySendError::Closed(_)) => break 'capture,
            }
            // Restore capacity for next iteration after mem::take.
            if scratch_pcm.capacity() < frame_samples {
                scratch_pcm.reserve(frame_samples);
            }
            if scratch_resample.capacity() < frame_samples {
                scratch_resample.reserve(frame_samples);
            }
        }

        capture_client.read_from_device_to_deque(&mut sample_queue)?;
        wait_for_audio(&stream.wait_mode);
    }

    audio_client.stop_stream()?;
    Ok(())
}
