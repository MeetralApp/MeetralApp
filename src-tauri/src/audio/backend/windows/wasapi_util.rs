use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use wasapi::{
    initialize_mta, AudioClient, Device, Direction, Handle, SampleType, StreamMode, WasapiError,
    WaveFormat,
};

use crate::config::INPUT_SAMPLE_RATE;

use super::super::pcm::{self, PcmSampleFormat};

const AUDCLNT_E_DEVICE_IN_USE: u32 = 0x8889000A;

static DEVICE_OPEN_LOCK: Mutex<()> = Mutex::new(());

pub fn device_open_lock() -> std::sync::MutexGuard<'static, ()> {
    DEVICE_OPEN_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub enum WaitMode {
    Event(Handle),
    Polling { sleep_ms: u64 },
}

pub struct StreamSetup {
    pub format: WaveFormat,
    pub blockalign: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub wait_mode: WaitMode,
}

pub fn init_com() -> Result<()> {
    let _ = initialize_mta();
    Ok(())
}

pub fn is_device_in_use(err: &WasapiError) -> bool {
    matches!(
        err,
        WasapiError::Windows(e) if e.code().0 as u32 == AUDCLNT_E_DEVICE_IN_USE
    )
}

pub fn open_stream(
    device: &Device,
    direction: &Direction,
    device_name: &str,
) -> Result<(AudioClient, StreamSetup)> {
    let _open_guard = device_open_lock();

    let mut last_error: Option<WasapiError> = None;

    for attempt in 0..3 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(200 * attempt as u64));
        }

        let mut audio_client = device.get_iaudioclient()?;
        let desired = preferred_format(&audio_client);
        let (def_time, _) = audio_client.get_device_period()?;

        match try_open_events(&mut audio_client, direction, &desired, def_time) {
            Ok(wait_mode) => {
                let setup = build_stream_setup(&audio_client, desired, wait_mode);
                return Ok((audio_client, setup));
            }
            Err(e) if is_device_in_use(&e) || matches!(e, WasapiError::EventTimeout) => {
                last_error = Some(e);
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    "WASAPI event mode failed for {device_name}: {e}, trying polling mode"
                );
                last_error = Some(e);
                break;
            }
        }
    }

    for attempt in 0..3 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(200 * attempt as u64));
        }

        let mut audio_client = device.get_iaudioclient()?;
        let desired = preferred_format(&audio_client);
        let (def_time, _) = audio_client.get_device_period()?;

        match try_open_polling(&mut audio_client, direction, &desired, def_time) {
            Ok(()) => {
                let sleep_ms = polling_sleep_ms(&audio_client, &desired);
                let setup =
                    build_stream_setup(&audio_client, desired, WaitMode::Polling { sleep_ms });
                return Ok((audio_client, setup));
            }
            Err(e) => last_error = Some(e),
        }
    }

    Err(anyhow!(
        "failed to open {device_name}: {}",
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown".into())
    ))
}

fn preferred_format(audio_client: &AudioClient) -> WaveFormat {
    audio_client.get_mixformat().unwrap_or_else(|_| {
        WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            INPUT_SAMPLE_RATE as usize,
            1,
            None,
        )
    })
}

fn try_open_events(
    audio_client: &mut AudioClient,
    direction: &Direction,
    format: &WaveFormat,
    buffer_duration_hns: i64,
) -> Result<WaitMode, WasapiError> {
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns,
    };
    audio_client.initialize_client(format, direction, &mode)?;
    let event = audio_client.set_get_eventhandle()?;
    Ok(WaitMode::Event(event))
}

fn try_open_polling(
    audio_client: &mut AudioClient,
    direction: &Direction,
    format: &WaveFormat,
    buffer_duration_hns: i64,
) -> Result<(), WasapiError> {
    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns,
    };
    audio_client.initialize_client(format, direction, &mode)
}

fn build_stream_setup(
    _audio_client: &AudioClient,
    format: WaveFormat,
    wait_mode: WaitMode,
) -> StreamSetup {
    StreamSetup {
        blockalign: format.get_blockalign(),
        sample_rate: format.get_samplespersec(),
        channels: format.get_nchannels(),
        format,
        wait_mode,
    }
}

fn polling_sleep_ms(audio_client: &AudioClient, format: &WaveFormat) -> u64 {
    let buffer_frames = audio_client.get_buffer_size().unwrap_or(480) as u64;
    let sample_rate = format.get_samplespersec().max(1) as u64;
    (500 * buffer_frames / sample_rate).max(5)
}

pub fn wait_for_audio(wait_mode: &WaitMode) {
    match wait_mode {
        WaitMode::Event(event) => {
            let _ = event.wait_for_event(500);
        }
        WaitMode::Polling { sleep_ms } => thread::sleep(Duration::from_millis(*sleep_ms)),
    }
}

pub fn wait_for_audio_paced(wait_mode: &WaitMode, min_sleep: Duration) {
    match wait_mode {
        WaitMode::Event(event) => {
            let timeout_ms = min_sleep.as_millis().min(500) as u32;
            let _ = event.wait_for_event(timeout_ms.max(1));
        }
        WaitMode::Polling { .. } => thread::sleep(min_sleep),
    }
}

fn pcm_format(sample_type: &SampleType) -> PcmSampleFormat {
    match sample_type {
        SampleType::Float => PcmSampleFormat::Float32,
        SampleType::Int => PcmSampleFormat::Int16,
    }
}

pub fn bytes_to_mono_i16(bytes: &[u8], channels: usize, sample_type: &SampleType) -> Vec<i16> {
    pcm::bytes_to_mono_i16(bytes, channels, pcm_format(sample_type))
}

pub fn bytes_to_mono_i16_into(
    bytes: &[u8],
    channels: usize,
    sample_type: &SampleType,
    out: &mut Vec<i16>,
) {
    pcm::bytes_to_mono_i16_into(bytes, channels, pcm_format(sample_type), out)
}

pub fn mono_i16_to_device_bytes(
    samples: &[i16],
    channels: u16,
    sample_type: &SampleType,
) -> Vec<u8> {
    pcm::mono_i16_to_device_bytes(samples, channels, pcm_format(sample_type))
}

pub fn mono_i16_to_device_bytes_into(
    samples: &[i16],
    channels: u16,
    sample_type: &SampleType,
    out: &mut Vec<u8>,
) {
    pcm::mono_i16_to_device_bytes_into(samples, channels, pcm_format(sample_type), out)
}
