//! Hot-plug / sample-rate listeners for Core Audio devices.

use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use objc2_core_audio::{
    kAudioHardwarePropertyDefaultInputDevice, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioHardwarePropertyDevices, AudioObjectPropertyListenerProc,
};

use super::super::super::capture::{AudioFaultEvent, AudioFaultSender};
use super::super::super::device_monitor::{DeviceChangeEvent, DeviceChangeSender};
use super::hal_sys::{self, DeviceId};

pub struct DeviceAliveListener {
    device_id: DeviceId,
    listener: AudioObjectPropertyListenerProc,
    client_ptr: *mut ListenerClient,
}

struct ListenerClient {
    device_id: String,
    device_name: String,
    fault_tx: AudioFaultSender,
}

impl DeviceAliveListener {
    pub fn register(
        device_id: DeviceId,
        uid: &str,
        name: &str,
        fault_tx: AudioFaultSender,
    ) -> anyhow::Result<Self> {
        let client = Box::new(ListenerClient {
            device_id: uid.to_string(),
            device_name: name.to_string(),
            fault_tx,
        });
        let client_ptr = Box::into_raw(client);
        let listener: AudioObjectPropertyListenerProc = Some(device_alive_listener);

        unsafe {
            hal_sys::add_device_alive_listener(device_id, listener, client_ptr.cast())?;
        }

        Ok(Self {
            device_id,
            listener,
            client_ptr,
        })
    }
}

impl Drop for DeviceAliveListener {
    fn drop(&mut self) {
        unsafe {
            let _ = hal_sys::remove_device_alive_listener(
                self.device_id,
                self.listener,
                self.client_ptr.cast(),
            );
            drop(Box::from_raw(self.client_ptr));
        }
    }
}

unsafe extern "C-unwind" fn device_alive_listener(
    object_id: objc2_core_audio::AudioObjectID,
    _num_addresses: u32,
    _addresses: NonNull<objc2_core_audio::AudioObjectPropertyAddress>,
    client_data: *mut std::ffi::c_void,
) -> i32 {
    if client_data.is_null() {
        return objc2_core_audio::kAudioHardwareNoError;
    }
    let client = &*(client_data as *const ListenerClient);
    if let Ok(false) = hal_sys::device_is_alive(object_id) {
        crate::runtime::control_channel::try_send_control(
            &client.fault_tx,
            AudioFaultEvent {
                device_id: client.device_id.clone(),
                device_name: client.device_name.clone(),
                reason: "device disconnected".to_string(),
            },
            "audio-fault",
        );
    }
    objc2_core_audio::kAudioHardwareNoError
}

/// System-object selectors that affect the device catalog: hardware add /
/// remove and default-device changes (empty device roles resolve to defaults).
const DEVICE_LIST_SELECTORS: [u32; 3] = [
    kAudioHardwarePropertyDevices,
    kAudioHardwarePropertyDefaultInputDevice,
    kAudioHardwarePropertyDefaultOutputDevice,
];

/// Wakes the engine watchdog when the Core Audio device catalog changes.
pub struct DeviceListListener {
    listener: AudioObjectPropertyListenerProc,
    client_ptr: *mut DeviceListClient,
}

struct DeviceListClient {
    tx: DeviceChangeSender,
}

// SAFETY: AudioObjectAdd/RemovePropertyListener are thread-safe; the client
// box is dereferenced only by callbacks while registered, and Drop removes all
// registrations before freeing it (same contract as DeviceAliveListener).
unsafe impl Send for DeviceListListener {}

impl DeviceListListener {
    pub fn register(tx: DeviceChangeSender) -> anyhow::Result<Self> {
        let client = Box::new(DeviceListClient { tx });
        let client_ptr = Box::into_raw(client);
        let listener: AudioObjectPropertyListenerProc = Some(device_list_listener);

        let registered: anyhow::Result<()> = (|| {
            for selector in DEVICE_LIST_SELECTORS {
                // SAFETY: `listener` is a valid proc and `client_ptr` points to
                // a live box kept until Drop unregisters all selectors.
                unsafe {
                    hal_sys::add_system_property_listener(selector, listener, client_ptr.cast())?;
                }
            }
            Ok(())
        })();

        if let Err(error) = registered {
            // SAFETY: unwind any partial registrations before freeing the box;
            // removing an unregistered selector is a harmless no-op error.
            unsafe {
                for selector in DEVICE_LIST_SELECTORS {
                    let _ = hal_sys::remove_system_property_listener(
                        selector,
                        listener,
                        client_ptr.cast(),
                    );
                }
                drop(Box::from_raw(client_ptr));
            }
            return Err(error);
        }

        Ok(Self {
            listener,
            client_ptr,
        })
    }
}

impl Drop for DeviceListListener {
    fn drop(&mut self) {
        // SAFETY: removes exactly what `register` added; callbacks cannot fire
        // after removal, so freeing the client box afterwards is sound.
        unsafe {
            for selector in DEVICE_LIST_SELECTORS {
                let _ = hal_sys::remove_system_property_listener(
                    selector,
                    self.listener,
                    self.client_ptr.cast(),
                );
            }
            drop(Box::from_raw(self.client_ptr));
        }
    }
}

unsafe extern "C-unwind" fn device_list_listener(
    _object_id: objc2_core_audio::AudioObjectID,
    _num_addresses: u32,
    _addresses: NonNull<objc2_core_audio::AudioObjectPropertyAddress>,
    client_data: *mut std::ffi::c_void,
) -> i32 {
    if client_data.is_null() {
        return objc2_core_audio::kAudioHardwareNoError;
    }
    // SAFETY: `client_data` is the `DeviceListClient` box created in
    // `register`, alive until the listener is unregistered.
    let client = &*(client_data as *const DeviceListClient);
    let _ = client.tx.send(DeviceChangeEvent);
    objc2_core_audio::kAudioHardwareNoError
}

/// Tracks `kAudioDevicePropertyNominalSampleRate` into a shared atomic for the
/// playback IO proc (Bluetooth HFP often flips 44.1/48 kHz → 16 kHz after mic opens).
pub struct SampleRateListener {
    device_id: DeviceId,
    listener: AudioObjectPropertyListenerProc,
    client_ptr: *mut SampleRateClient,
}

struct SampleRateClient {
    rate: Arc<AtomicU32>,
}

impl SampleRateListener {
    pub fn register(device_id: DeviceId, rate: Arc<AtomicU32>) -> anyhow::Result<Self> {
        let client = Box::new(SampleRateClient { rate });
        let client_ptr = Box::into_raw(client);
        let listener: AudioObjectPropertyListenerProc = Some(sample_rate_listener);

        unsafe {
            hal_sys::add_nominal_sample_rate_listener(device_id, listener, client_ptr.cast())?;
        }

        Ok(Self {
            device_id,
            listener,
            client_ptr,
        })
    }
}

impl Drop for SampleRateListener {
    fn drop(&mut self) {
        unsafe {
            let _ = hal_sys::remove_nominal_sample_rate_listener(
                self.device_id,
                self.listener,
                self.client_ptr.cast(),
            );
            drop(Box::from_raw(self.client_ptr));
        }
    }
}

unsafe extern "C-unwind" fn sample_rate_listener(
    object_id: objc2_core_audio::AudioObjectID,
    _num_addresses: u32,
    _addresses: NonNull<objc2_core_audio::AudioObjectPropertyAddress>,
    client_data: *mut std::ffi::c_void,
) -> i32 {
    if client_data.is_null() {
        return objc2_core_audio::kAudioHardwareNoError;
    }
    let client = &*(client_data as *const SampleRateClient);
    if let Ok(rate) = hal_sys::device_sample_rate(object_id) {
        let rate = rate.round().max(1.0) as u32;
        let prev = client.rate.swap(rate, Ordering::SeqCst);
        if prev != rate {
            tracing::info!("playback device sample rate changed: {prev} → {rate} Hz");
        }
    }
    objc2_core_audio::kAudioHardwareNoError
}
