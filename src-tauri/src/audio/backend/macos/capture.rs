use std::collections::VecDeque;
use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::{anyhow, Result};
use objc2_core_audio::{
    kAudioHardwareNoError, AudioDeviceCreateIOProcID, AudioDeviceDestroyIOProcID,
    AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop,
};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};
use tokio::sync::mpsc;

use super::super::super::capture::{
    AudioFaultEvent, AudioFaultSender, CaptureHandle, CaptureHeartbeat, CaptureSender,
};
use super::super::super::device::{resolve_role_device, AudioRole, ResolvedDevice};
use super::super::super::resampler::resample_mono_to_rate_into;
use super::format::{self, MonoReadScratch};
use super::hal_device::resolve_hal_device;
use super::listener::DeviceAliveListener;
use crate::config::{AppConfig, FRAME_MS, INPUT_SAMPLE_RATE};

struct CaptureClient {
    tx: CaptureSender,
    heartbeat: CaptureHeartbeat,
    accum: VecDeque<i16>,
    frame_samples: usize,
    device_rate: u32,
    scratch_capture: Vec<i16>,
    scratch_resample: Vec<i16>,
    scratch_frame: Vec<i16>,
    read_scratch: MonoReadScratch,
}

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
    let fault_tx_worker = fault_tx.clone();
    let thread = std::thread::Builder::new()
        .name("coreaudio-capture".into())
        .spawn(move || {
            if let Err(e) =
                capture_worker(&device, stop_flag, tx, heartbeat_worker, fault_tx_worker)
            {
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
    fault_tx: Option<AudioFaultSender>,
) -> Result<()> {
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

        match run_capture_session(resolved, &stop, &tx, &heartbeat, fault_tx.as_ref()) {
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

unsafe extern "C-unwind" fn capture_io_proc(
    _device: objc2_core_audio::AudioObjectID,
    _now: NonNull<AudioTimeStamp>,
    input_data: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>,
    _output_data: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>,
    client_data: *mut std::ffi::c_void,
) -> i32 {
    if client_data.is_null() {
        return kAudioHardwareNoError;
    }
    let client = &mut *(client_data as *mut CaptureClient);
    format::read_input_mono_i16_into(
        input_data.as_ref(),
        &mut client.scratch_capture,
        &mut client.read_scratch,
    );
    if client.scratch_capture.is_empty() {
        return kAudioHardwareNoError;
    }

    let pcm: &[i16] = if client.device_rate != INPUT_SAMPLE_RATE {
        resample_mono_to_rate_into(
            &client.scratch_capture,
            client.device_rate,
            INPUT_SAMPLE_RATE,
            &mut client.scratch_resample,
        );
        &client.scratch_resample
    } else {
        &client.scratch_capture
    };

    client.accum.extend(pcm.iter().copied());
    while client.accum.len() >= client.frame_samples {
        client.scratch_frame.clear();
        client
            .scratch_frame
            .extend(client.accum.drain(..client.frame_samples));
        let frame = std::mem::replace(
            &mut client.scratch_frame,
            Vec::with_capacity(client.frame_samples),
        );
        match client.tx.try_send(frame) {
            Ok(()) => client.heartbeat.touch(),
            Err(mpsc::error::TrySendError::Full(_)) => client.heartbeat.touch(),
            Err(mpsc::error::TrySendError::Closed(_)) => return kAudioHardwareNoError,
        }
    }
    kAudioHardwareNoError
}

fn run_capture_session(
    resolved: &ResolvedDevice,
    stop: &Arc<AtomicBool>,
    tx: &CaptureSender,
    heartbeat: &CaptureHeartbeat,
    fault_tx: Option<&AudioFaultSender>,
) -> Result<()> {
    let hal = resolve_hal_device(resolved)?;
    let device_rate = super::hal_sys::device_sample_rate(hal.object_id)
        .unwrap_or(INPUT_SAMPLE_RATE as f64)
        .round() as u32;
    let device_rate = device_rate.max(1);

    let frame_samples = (INPUT_SAMPLE_RATE * FRAME_MS / 1000) as usize;
    let client = Box::new(CaptureClient {
        tx: tx.clone(),
        heartbeat: heartbeat.clone(),
        accum: VecDeque::with_capacity(frame_samples * 4),
        frame_samples,
        device_rate,
        scratch_capture: Vec::with_capacity(2048),
        scratch_resample: Vec::with_capacity(2048),
        scratch_frame: Vec::with_capacity(frame_samples),
        read_scratch: MonoReadScratch::with_capacity(16),
    });

    let client_ptr = Box::into_raw(client);
    let mut proc_id: AudioDeviceIOProcID = None;

    let status = unsafe {
        AudioDeviceCreateIOProcID(
            hal.object_id,
            Some(capture_io_proc),
            client_ptr.cast(),
            NonNull::from(&mut proc_id),
        )
    };
    if status != kAudioHardwareNoError {
        unsafe {
            drop(Box::from_raw(client_ptr));
        }
        return Err(anyhow!("AudioDeviceCreateIOProcID failed: {status}"));
    }

    let label = format!("{} ({})", hal.name, hal.uid);
    let start_status = unsafe { AudioDeviceStart(hal.object_id, proc_id) };
    if start_status != kAudioHardwareNoError {
        unsafe {
            let _ = AudioDeviceDestroyIOProcID(hal.object_id, proc_id);
            drop(Box::from_raw(client_ptr));
        }
        return Err(anyhow!("AudioDeviceStart failed: {start_status}"));
    }

    tracing::info!("capture opened: {} ({} Hz nominal)", label, device_rate);

    let _alive_listener = fault_tx.and_then(|tx| {
        DeviceAliveListener::register(hal.object_id, &hal.uid, &hal.name, tx.clone())
            .map_err(|e| {
                tracing::warn!(
                    "failed to register device alive listener for {}: {e:#}",
                    hal.uid
                );
                e
            })
            .ok()
    });

    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(10));
    }

    unsafe {
        let _ = AudioDeviceStop(hal.object_id, proc_id);
        let _ = AudioDeviceDestroyIOProcID(hal.object_id, proc_id);
        drop(Box::from_raw(client_ptr));
    }
    Ok(())
}
