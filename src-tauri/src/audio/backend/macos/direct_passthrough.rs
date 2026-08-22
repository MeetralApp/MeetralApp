// Single-threaded capture → playback passthrough for Direct mode on macOS.
//
// One management thread registers Core Audio IO procs on capture and playback
// devices. Capture and playback callbacks share a mutex-protected queue so
// audio is relayed at device callback rate without tokio/mpsc relay threads.
//
// Same-rate path stays in float32 (no f32↔i16 round-trip). Rate-mismatch uses
// mono i16 + one-hop linear resample.
//
// Cold-start: open capture first, settle HAL nominal rates (HFP/aggregate), then
// start playback. Live SampleRateListener + poll reconfigure if rates flip later.

use std::collections::VecDeque;
use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use objc2_core_audio::{
    kAudioHardwareNoError, AudioDeviceCreateIOProcID, AudioDeviceDestroyIOProcID,
    AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop,
};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};

use super::super::super::capture::{
    AudioFaultEvent, AudioFaultSender, CaptureHandle, CaptureHeartbeat,
};
use super::super::super::device::ResolvedDevice;
use super::format::{self, MonoReadScratch};
use super::hal_device::resolve_hal_device;
use super::listener::{DeviceAliveListener, SampleRateListener};
use crate::audio::gate_pcm_in_place;
use crate::config::INPUT_SAMPLE_RATE;

/// Smaller than the translate pipeline frame (100 ms) for smoother local passthrough.
const DIRECT_FRAME_MS: u32 = 20;
/// Max queued playback audio before dropping oldest frames (~80 ms at 20 ms/frame).
const MAX_RENDER_PENDING_FRAMES: usize = 4;
/// Wait after capture start for Bluetooth HFP / aggregates to flip nominal rate.
const RATE_SETTLE_TIMEOUT: Duration = Duration::from_millis(250);
const RATE_SETTLE_POLL: Duration = Duration::from_millis(50);
/// Backup HAL rate poll on the management thread (listeners sometimes miss aggregates).
const RATE_LIVE_POLL: Duration = Duration::from_millis(100);

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
                mute_gate,
                heartbeat_worker,
                fault_tx.clone(),
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
    mute_gate: Arc<AtomicBool>,
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
            mute_gate.clone(),
            heartbeat.clone(),
            fault_tx.as_ref(),
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

fn gate_float_in_place(muted: &AtomicBool, pcm: &mut [f32]) {
    if muted.load(Ordering::Relaxed) {
        pcm.fill(0.0);
    }
}

fn read_device_rate(device_id: u32) -> u32 {
    super::hal_sys::device_sample_rate(device_id)
        .unwrap_or(INPUT_SAMPLE_RATE as f64)
        .round()
        .max(1.0) as u32
}

fn frame_samples_for_rate(rate: u32) -> usize {
    (rate as u64 * DIRECT_FRAME_MS as u64 / 1000).max(1) as usize
}

/// Same-rate float32 relay (no i16 conversion).
struct FloatRelay {
    capture_accum: VecDeque<f32>,
    render_pending: VecDeque<f32>,
    scratch_capture: Vec<f32>,
    scratch_frame: Vec<f32>,
    scratch_output: Vec<f32>,
}

/// Rate-mismatch relay via mono i16 + linear resample.
struct I16Relay {
    capture_rate: u32,
    playback_rate: u32,
    capture_accum: VecDeque<i16>,
    render_pending: VecDeque<i16>,
    scratch_capture: Vec<i16>,
    scratch_frame: Vec<i16>,
    scratch_resample: Vec<i16>,
    scratch_output: Vec<i16>,
}

enum RelayPath {
    Float(FloatRelay),
    I16(I16Relay),
}

fn build_relay_path(
    capture_rate: u32,
    playback_rate: u32,
    frame_samples_input: usize,
    playback_frame_samples: usize,
    max_render_pending_samples: usize,
) -> RelayPath {
    if capture_rate == playback_rate {
        RelayPath::Float(FloatRelay {
            capture_accum: VecDeque::with_capacity(frame_samples_input * 2),
            render_pending: VecDeque::with_capacity(
                max_render_pending_samples + playback_frame_samples,
            ),
            scratch_capture: Vec::with_capacity(2048),
            scratch_frame: Vec::with_capacity(frame_samples_input),
            scratch_output: Vec::with_capacity(2048),
        })
    } else {
        RelayPath::I16(I16Relay {
            capture_rate,
            playback_rate,
            capture_accum: VecDeque::with_capacity(frame_samples_input * 2),
            render_pending: VecDeque::with_capacity(
                max_render_pending_samples + playback_frame_samples,
            ),
            scratch_capture: Vec::with_capacity(2048),
            scratch_frame: Vec::with_capacity(frame_samples_input),
            scratch_resample: Vec::with_capacity(playback_frame_samples),
            scratch_output: Vec::with_capacity(2048),
        })
    }
}

struct PassthroughClient {
    mute_gate: Arc<AtomicBool>,
    heartbeat: CaptureHeartbeat,
    capture_rate: u32,
    playback_rate: u32,
    frame_samples_input: usize,
    playback_frame_samples: usize,
    max_render_pending_samples: usize,
    path: RelayPath,
    read_scratch: MonoReadScratch,
}

impl PassthroughClient {
    fn new(
        mute_gate: Arc<AtomicBool>,
        heartbeat: CaptureHeartbeat,
        capture_rate: u32,
        playback_rate: u32,
    ) -> Self {
        let capture_rate = capture_rate.max(1);
        let playback_rate = playback_rate.max(1);
        let frame_samples_input = frame_samples_for_rate(capture_rate);
        let playback_frame_samples = frame_samples_for_rate(playback_rate);
        let max_render_pending_samples = playback_frame_samples * MAX_RENDER_PENDING_FRAMES;
        let path = build_relay_path(
            capture_rate,
            playback_rate,
            frame_samples_input,
            playback_frame_samples,
            max_render_pending_samples,
        );
        Self {
            mute_gate,
            heartbeat,
            capture_rate,
            playback_rate,
            frame_samples_input,
            playback_frame_samples,
            max_render_pending_samples,
            path,
            read_scratch: MonoReadScratch::with_capacity(16),
        }
    }

    /// Rebuild path / frame sizes when HAL nominal rates change. Clears queues.
    fn reconfigure_rates(&mut self, capture_rate: u32, playback_rate: u32) -> bool {
        let capture_rate = capture_rate.max(1);
        let playback_rate = playback_rate.max(1);
        if self.capture_rate == capture_rate && self.playback_rate == playback_rate {
            return false;
        }
        let prev_cap = self.capture_rate;
        let prev_play = self.playback_rate;
        let prev_path = self.path_label();

        self.capture_rate = capture_rate;
        self.playback_rate = playback_rate;
        self.frame_samples_input = frame_samples_for_rate(capture_rate);
        self.playback_frame_samples = frame_samples_for_rate(playback_rate);
        self.max_render_pending_samples = self.playback_frame_samples * MAX_RENDER_PENDING_FRAMES;
        self.path = build_relay_path(
            capture_rate,
            playback_rate,
            self.frame_samples_input,
            self.playback_frame_samples,
            self.max_render_pending_samples,
        );

        tracing::info!(
            "direct passthrough rates reconfigured: capture {prev_cap}→{capture_rate} Hz, playback {prev_play}→{playback_rate} Hz, path {prev_path}→{}",
            self.path_label()
        );
        true
    }

    fn ingest_capture(&mut self, input: &AudioBufferList) {
        let PassthroughClient {
            mute_gate,
            heartbeat,
            frame_samples_input,
            playback_frame_samples,
            max_render_pending_samples,
            path,
            read_scratch,
            ..
        } = self;

        match path {
            RelayPath::Float(relay) => {
                unsafe {
                    format::read_input_mono_f32_into(
                        input,
                        &mut relay.scratch_capture,
                        read_scratch,
                    );
                }
                if relay.scratch_capture.is_empty() {
                    return;
                }
                relay
                    .capture_accum
                    .extend(relay.scratch_capture.iter().copied());
                while relay.capture_accum.len() >= *frame_samples_input {
                    relay.scratch_frame.clear();
                    relay
                        .scratch_frame
                        .extend(relay.capture_accum.drain(..*frame_samples_input));
                    gate_float_in_place(mute_gate.as_ref(), &mut relay.scratch_frame);
                    heartbeat.touch();
                    relay
                        .render_pending
                        .extend(relay.scratch_frame.iter().copied());
                    trim_render_backlog(
                        &mut relay.render_pending,
                        *max_render_pending_samples,
                        *playback_frame_samples,
                    );
                }
            }
            RelayPath::I16(relay) => {
                unsafe {
                    format::read_input_mono_i16_into(
                        input,
                        &mut relay.scratch_capture,
                        read_scratch,
                    );
                }
                if relay.scratch_capture.is_empty() {
                    return;
                }
                relay
                    .capture_accum
                    .extend(relay.scratch_capture.iter().copied());
                while relay.capture_accum.len() >= *frame_samples_input {
                    relay.scratch_frame.clear();
                    relay
                        .scratch_frame
                        .extend(relay.capture_accum.drain(..*frame_samples_input));
                    gate_pcm_in_place(mute_gate.as_ref(), &mut relay.scratch_frame);
                    heartbeat.touch();
                    crate::audio::resampler::resample_mono_to_rate_into(
                        &relay.scratch_frame,
                        relay.capture_rate,
                        relay.playback_rate,
                        &mut relay.scratch_resample,
                    );
                    relay
                        .render_pending
                        .extend(relay.scratch_resample.iter().copied());
                    trim_render_backlog(
                        &mut relay.render_pending,
                        *max_render_pending_samples,
                        *playback_frame_samples,
                    );
                }
            }
        }
    }

    fn fill_playback_output(&mut self, output: &mut AudioBufferList) {
        let frames_needed = format::output_frames_needed(output);
        match &mut self.path {
            RelayPath::Float(relay) => {
                relay.scratch_output.resize(frames_needed, 0.0);
                let available = frames_needed.min(relay.render_pending.len());
                for (i, sample) in relay.render_pending.drain(..available).enumerate() {
                    relay.scratch_output[i] = sample;
                }
                if available < frames_needed {
                    relay.scratch_output[available..].fill(0.0);
                }
                unsafe {
                    format::write_output_mono_f32(output, &relay.scratch_output);
                }
            }
            RelayPath::I16(relay) => {
                relay.scratch_output.resize(frames_needed, 0);
                let available = frames_needed.min(relay.render_pending.len());
                for (i, sample) in relay.render_pending.drain(..available).enumerate() {
                    relay.scratch_output[i] = sample;
                }
                if available < frames_needed {
                    relay.scratch_output[available..].fill(0);
                }
                unsafe {
                    format::write_output_mono_i16(output, &relay.scratch_output);
                }
            }
        }
    }

    fn path_label(&self) -> &'static str {
        match self.path {
            RelayPath::Float(_) => "float32",
            RelayPath::I16(_) => "i16-resample",
        }
    }
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
    let client_mutex = &*(client_data as *const Mutex<PassthroughClient>);
    let Ok(mut client) = client_mutex.try_lock() else {
        return kAudioHardwareNoError;
    };
    client.ingest_capture(input_data.as_ref());
    kAudioHardwareNoError
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
    let output = output_data.as_mut();
    let client_mutex = &*(client_data as *const Mutex<PassthroughClient>);
    let Ok(mut client) = client_mutex.try_lock() else {
        unsafe {
            format::write_silence_output(output);
        }
        return kAudioHardwareNoError;
    };
    client.fill_playback_output(output);
    kAudioHardwareNoError
}

/// Poll until capture/playback nominal rates are stable for two consecutive reads,
/// or until timeout (Bluetooth HFP often flips after mic opens).
fn settle_sample_rates(
    capture_device_id: u32,
    playback_device_id: u32,
    stop: &AtomicBool,
    capture_rate_atomic: &AtomicU32,
    playback_rate_atomic: &AtomicU32,
) -> (u32, u32) {
    let mut last_cap = read_device_rate(capture_device_id);
    let mut last_play = read_device_rate(playback_device_id);
    capture_rate_atomic.store(last_cap, Ordering::Relaxed);
    playback_rate_atomic.store(last_play, Ordering::Relaxed);

    let deadline = Instant::now() + RATE_SETTLE_TIMEOUT;
    let mut stable_streak = 0u32;

    while Instant::now() < deadline {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(RATE_SETTLE_POLL);
        let cap = read_device_rate(capture_device_id);
        let play = read_device_rate(playback_device_id);
        capture_rate_atomic.store(cap, Ordering::Relaxed);
        playback_rate_atomic.store(play, Ordering::Relaxed);

        if cap == last_cap && play == last_play {
            stable_streak += 1;
            if stable_streak >= 2 {
                break;
            }
        } else {
            if cap != last_cap || play != last_play {
                tracing::info!(
                    "direct passthrough settling rates: capture {last_cap}→{cap} Hz, playback {last_play}→{play} Hz"
                );
            }
            last_cap = cap;
            last_play = play;
            stable_streak = 0;
        }
    }

    (last_cap.max(1), last_play.max(1))
}

fn run_passthrough_session(
    capture_resolved: &ResolvedDevice,
    playback_resolved: &ResolvedDevice,
    stop: &Arc<AtomicBool>,
    mute_gate: Arc<AtomicBool>,
    heartbeat: CaptureHeartbeat,
    fault_tx: Option<&AudioFaultSender>,
) -> Result<()> {
    let capture_hal = resolve_hal_device(capture_resolved)?;
    let playback_hal = resolve_hal_device(playback_resolved)?;

    let initial_capture_rate = read_device_rate(capture_hal.object_id);
    let initial_playback_rate = read_device_rate(playback_hal.object_id);

    let capture_rate_atomic = Arc::new(AtomicU32::new(initial_capture_rate));
    let playback_rate_atomic = Arc::new(AtomicU32::new(initial_playback_rate));

    let client = Mutex::new(PassthroughClient::new(
        mute_gate,
        heartbeat.clone(),
        initial_capture_rate,
        initial_playback_rate,
    ));
    let client_ptr = Box::into_raw(Box::new(client));

    let mut capture_proc_id: AudioDeviceIOProcID = None;
    let capture_status = unsafe {
        AudioDeviceCreateIOProcID(
            capture_hal.object_id,
            Some(capture_io_proc),
            client_ptr.cast(),
            NonNull::from(&mut capture_proc_id),
        )
    };
    if capture_status != kAudioHardwareNoError {
        unsafe {
            drop(Box::from_raw(client_ptr));
        }
        return Err(anyhow!(
            "capture AudioDeviceCreateIOProcID failed: {capture_status}"
        ));
    }

    let mut playback_proc_id: AudioDeviceIOProcID = None;
    let playback_status = unsafe {
        AudioDeviceCreateIOProcID(
            playback_hal.object_id,
            Some(playback_io_proc),
            client_ptr.cast(),
            NonNull::from(&mut playback_proc_id),
        )
    };
    if playback_status != kAudioHardwareNoError {
        unsafe {
            let _ = AudioDeviceDestroyIOProcID(capture_hal.object_id, capture_proc_id);
            drop(Box::from_raw(client_ptr));
        }
        return Err(anyhow!(
            "playback AudioDeviceCreateIOProcID failed: {playback_status}"
        ));
    }

    let capture_label = format!("{} ({})", capture_hal.name, capture_hal.uid);
    let playback_label = format!("{} ({})", playback_hal.name, playback_hal.uid);

    // Capture-first so HFP / aggregates can flip nominal rate before we lock the path.
    let capture_start = unsafe { AudioDeviceStart(capture_hal.object_id, capture_proc_id) };
    if capture_start != kAudioHardwareNoError {
        unsafe {
            let _ = AudioDeviceDestroyIOProcID(capture_hal.object_id, capture_proc_id);
            let _ = AudioDeviceDestroyIOProcID(playback_hal.object_id, playback_proc_id);
            drop(Box::from_raw(client_ptr));
        }
        return Err(anyhow!("capture AudioDeviceStart failed: {capture_start}"));
    }

    let (settled_cap, settled_play) = settle_sample_rates(
        capture_hal.object_id,
        playback_hal.object_id,
        stop,
        &capture_rate_atomic,
        &playback_rate_atomic,
    );

    {
        let client = unsafe { &*client_ptr };
        if let Ok(mut guard) = client.lock() {
            guard.reconfigure_rates(settled_cap, settled_play);
        }
    }

    let path_label = {
        let client = unsafe { &*client_ptr };
        client.lock().map(|g| g.path_label()).unwrap_or("unknown")
    };

    let playback_start = unsafe { AudioDeviceStart(playback_hal.object_id, playback_proc_id) };
    if playback_start != kAudioHardwareNoError {
        unsafe {
            let _ = AudioDeviceStop(capture_hal.object_id, capture_proc_id);
            let _ = AudioDeviceDestroyIOProcID(capture_hal.object_id, capture_proc_id);
            let _ = AudioDeviceDestroyIOProcID(playback_hal.object_id, playback_proc_id);
            drop(Box::from_raw(client_ptr));
        }
        return Err(anyhow!(
            "playback AudioDeviceStart failed: {playback_start}"
        ));
    }

    tracing::info!(
        "direct passthrough: {} → {} ({} ms frames, capture {} Hz, playback {} Hz, path={}, settled)",
        capture_label,
        playback_label,
        DIRECT_FRAME_MS,
        settled_cap,
        settled_play,
        path_label
    );

    let _capture_rate_listener =
        SampleRateListener::register(capture_hal.object_id, capture_rate_atomic.clone()).ok();
    let _playback_rate_listener =
        SampleRateListener::register(playback_hal.object_id, playback_rate_atomic.clone()).ok();

    let _alive_listener = fault_tx.and_then(|tx| {
        DeviceAliveListener::register(
            capture_hal.object_id,
            &capture_hal.uid,
            &capture_hal.name,
            tx.clone(),
        )
        .map_err(|e| {
            tracing::warn!(
                "failed to register device alive listener for {}: {e:#}",
                capture_hal.uid
            );
            e
        })
        .ok()
    });

    let mut last_hal_poll = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        if last_hal_poll.elapsed() >= RATE_LIVE_POLL {
            last_hal_poll = Instant::now();
            // Backup for aggregates that miss property listeners.
            let cap = read_device_rate(capture_hal.object_id);
            let play = read_device_rate(playback_hal.object_id);
            capture_rate_atomic.store(cap, Ordering::Relaxed);
            playback_rate_atomic.store(play, Ordering::Relaxed);
        }

        let live_cap = capture_rate_atomic.load(Ordering::Relaxed).max(1);
        let live_play = playback_rate_atomic.load(Ordering::Relaxed).max(1);
        let client = unsafe { &*client_ptr };
        if let Ok(mut guard) = client.try_lock() {
            guard.reconfigure_rates(live_cap, live_play);
        }

        std::thread::sleep(Duration::from_millis(DIRECT_FRAME_MS as u64));
    }

    drop(_capture_rate_listener);
    drop(_playback_rate_listener);

    unsafe {
        let _ = AudioDeviceStop(capture_hal.object_id, capture_proc_id);
        let _ = AudioDeviceStop(playback_hal.object_id, playback_proc_id);
        let _ = AudioDeviceDestroyIOProcID(capture_hal.object_id, capture_proc_id);
        let _ = AudioDeviceDestroyIOProcID(playback_hal.object_id, playback_proc_id);
        drop(Box::from_raw(client_ptr));
    }

    Ok(())
}

fn trim_render_backlog<T>(pending: &mut VecDeque<T>, max_samples: usize, frame_samples: usize) {
    if frame_samples == 0 {
        return;
    }
    let mut dropped_frames = 0u32;
    while pending.len() > max_samples {
        let drop = frame_samples.min(pending.len());
        pending.drain(..drop);
        dropped_frames += 1;
    }
    if dropped_frames > 0 {
        tracing::debug!(
            "direct passthrough: dropped {dropped_frames} stale render frame(s) (cap {max_samples} samples)"
        );
    }
}
