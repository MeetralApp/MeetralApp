use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use anyhow::{anyhow, Result};
use objc2_core_audio::{
    kAudioHardwareNoError, AudioDeviceCreateIOProcID, AudioDeviceDestroyIOProcID,
    AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop,
};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};
use tokio::sync::mpsc;

use super::super::super::device::{resolve_role_device, AudioRole, ResolvedDevice};
use super::super::super::playback::PlaybackHandle;
use super::super::super::playback_buffer::{
    PlaybackBufferConfig, PlaybackFillStatus, PlaybackRingBuffer,
};
use super::format;
use super::hal_device::resolve_hal_device;
use super::listener::SampleRateListener;
use crate::config::INPUT_SAMPLE_RATE;

struct PlaybackClient {
    ring: PlaybackRingBuffer,
    rx: mpsc::Receiver<Vec<i16>>,
    /// Latest Core Audio nominal rate (updated by [`SampleRateListener`]).
    device_rate: Arc<AtomicU32>,
    /// Reused each IO callback to avoid per-callback `Vec` allocation.
    scratch_i16: Vec<i16>,
}

pub fn start_playback(
    resolved: ResolvedDevice,
    rx: mpsc::Receiver<Vec<i16>>,
    buffer_config: PlaybackBufferConfig,
) -> Result<PlaybackHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let device = resolved.clone();

    let thread = std::thread::Builder::new()
        .name("coreaudio-playback".into())
        .spawn(move || {
            if let Err(e) = playback_worker(&device, stop_flag, rx, buffer_config) {
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
    rx: mpsc::Receiver<Vec<i16>>,
    buffer_config: PlaybackBufferConfig,
) -> Result<()> {
    run_playback_session(resolved, &stop, rx, buffer_config)
}

unsafe extern "C-unwind" fn playback_io_proc(
    _device: objc2_core_audio::AudioObjectID,
    _now: NonNull<AudioTimeStamp>,
    _input_data: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>,
    mut output_data: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>,
    client_data: *mut std::ffi::c_void,
) -> i32 {
    if client_data.is_null() {
        return kAudioHardwareNoError;
    }
    let client = &mut *(client_data as *mut PlaybackClient);
    let output = output_data.as_mut();

    // Bluetooth HFP (and Multi-Output aggregates) can flip nominal rate after
    // the mic opens — keep the ring's resample target in sync.
    let live_rate = client.device_rate.load(Ordering::Relaxed).max(1);
    if live_rate != client.ring.device_rate() {
        client.ring.set_device_rate(live_rate);
    }

    let frames_needed = format::output_frames_needed(output);

    // Realtime path: never sleep inside the Core Audio IO proc.
    match client
        .ring
        .ensure_device_samples(frames_needed, &mut client.rx)
    {
        PlaybackFillStatus::Disconnected if client.ring.is_empty() => {
            format::write_silence_output(output);
            return kAudioHardwareNoError;
        }
        _ => {}
    }

    client.scratch_i16.resize(frames_needed, 0);
    let n = client.ring.pop_device_samples_into(&mut client.scratch_i16);
    if n < frames_needed {
        client.scratch_i16[n..].fill(0);
    }
    format::write_output_mono_i16(output, &client.scratch_i16);
    kAudioHardwareNoError
}

fn run_playback_session(
    resolved: &ResolvedDevice,
    stop: &Arc<AtomicBool>,
    rx: mpsc::Receiver<Vec<i16>>,
    buffer_config: PlaybackBufferConfig,
) -> Result<()> {
    let hal = resolve_hal_device(resolved)?;
    let device_rate = super::hal_sys::device_sample_rate(hal.object_id)
        .unwrap_or(INPUT_SAMPLE_RATE as f64)
        .round() as u32;
    let device_rate = device_rate.max(1);
    let device_rate_atomic = Arc::new(AtomicU32::new(device_rate));

    let output_channels = output_channel_count(hal.object_id).unwrap_or(2);

    let rate_listener =
        SampleRateListener::register(hal.object_id, device_rate_atomic.clone()).ok();
    if rate_listener.is_none() {
        tracing::warn!(
            "playback sample-rate listener unavailable for {} ({})",
            hal.name,
            hal.uid
        );
    }

    let client = Box::new(PlaybackClient {
        ring: PlaybackRingBuffer::new(buffer_config, device_rate),
        rx,
        device_rate: device_rate_atomic,
        scratch_i16: Vec::with_capacity(2048),
    });

    let client_ptr = Box::into_raw(client);
    let mut proc_id: AudioDeviceIOProcID = None;

    let status = unsafe {
        AudioDeviceCreateIOProcID(
            hal.object_id,
            Some(playback_io_proc),
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

    tracing::info!(
        "playback opened: {} ({} Hz, {} ch, ring buffer)",
        label,
        device_rate,
        output_channels
    );

    while !stop.load(Ordering::SeqCst) {
        // Re-poll occasionally: some aggregate/Multi-Output devices change rate
        // without delivering a property listener callback promptly.
        if let Ok(rate) = super::hal_sys::device_sample_rate(hal.object_id) {
            let rate = rate.round().max(1.0) as u32;
            let client = unsafe { &*client_ptr };
            let prev = client.device_rate.swap(rate, Ordering::Relaxed);
            if prev != rate {
                tracing::info!("playback device sample rate polled: {prev} → {rate} Hz");
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    drop(rate_listener);

    unsafe {
        let _ = AudioDeviceStop(hal.object_id, proc_id);
        let _ = AudioDeviceDestroyIOProcID(hal.object_id, proc_id);
        drop(Box::from_raw(client_ptr));
    }
    Ok(())
}

fn output_channel_count(device_id: u32) -> Result<usize> {
    use std::ptr::null;

    use objc2_core_audio::{
        kAudioDevicePropertyStreamConfiguration, kAudioObjectPropertyElementWildcard,
        kAudioObjectPropertyScopeOutput, AudioObjectGetPropertyData,
        AudioObjectGetPropertyDataSize, AudioObjectPropertyAddress,
    };
    use objc2_core_audio_types::AudioBufferList;

    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyStreamConfiguration,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementWildcard,
    };

    let mut data_size = 0u32;
    unsafe {
        super::hal_sys::check_os_status(AudioObjectGetPropertyDataSize(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut data_size),
        ))?;
        let mut buffer = vec![0u8; data_size as usize];
        let mut size = data_size;
        super::hal_sys::check_os_status(AudioObjectGetPropertyData(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::new(buffer.as_mut_ptr()).unwrap().cast(),
        ))?;
        let list = &*(buffer.as_ptr() as *const AudioBufferList);
        let channels: u32 = (0..list.mNumberBuffers)
            .map(|i| list.mBuffers[i as usize].mNumberChannels)
            .sum();
        Ok(channels.max(1) as usize)
    }
}
