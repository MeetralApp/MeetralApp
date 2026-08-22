//! Low-level Core Audio HAL property helpers (objc2-core-audio).

use std::mem;
use std::ptr::{null, NonNull};

use anyhow::{anyhow, Context, Result};
use objc2_core_audio::{
    kAudioDevicePropertyDeviceIsAlive, kAudioDevicePropertyDeviceNameCFString,
    kAudioDevicePropertyDeviceUID, kAudioDevicePropertyNominalSampleRate,
    kAudioDevicePropertyStreamConfiguration, kAudioHardwareNoError,
    kAudioHardwarePropertyDefaultInputDevice, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioHardwarePropertyDevices, kAudioHardwarePropertyTranslateUIDToDevice,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyElementWildcard,
    kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeInput,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject, AudioDeviceID,
    AudioObjectAddPropertyListener, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertyListenerProc,
    AudioObjectRemovePropertyListener,
};
use objc2_core_audio_types::AudioBufferList;
use objc2_core_foundation::{CFRetained, CFString};

pub type DeviceId = AudioDeviceID;

pub fn check_os_status(status: i32) -> Result<()> {
    if status == kAudioHardwareNoError {
        Ok(())
    } else {
        Err(anyhow!("Core Audio OSStatus {status}"))
    }
}

pub fn system_object() -> AudioObjectID {
    kAudioObjectSystemObject as AudioObjectID
}

fn property_address(selector: u32, scope: u32, element: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

pub fn all_device_ids() -> Result<Vec<DeviceId>> {
    let address = property_address(
        kAudioHardwarePropertyDevices,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut data_size = 0u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyDataSize(
            system_object(),
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut data_size),
        ))?;
    }

    let count = data_size as usize / mem::size_of::<DeviceId>();
    let mut devices = vec![0u32; count];
    let mut size = data_size;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            system_object(),
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::new(devices.as_mut_ptr()).unwrap().cast(),
        ))?;
    }
    Ok(devices)
}

pub fn device_supports_input(device_id: DeviceId) -> Result<bool> {
    device_has_stream_channels(device_id, kAudioObjectPropertyScopeInput)
}

pub fn device_supports_output(device_id: DeviceId) -> Result<bool> {
    device_has_stream_channels(device_id, kAudioObjectPropertyScopeOutput)
}

fn device_has_stream_channels(device_id: DeviceId, scope: u32) -> Result<bool> {
    let address = property_address(
        kAudioDevicePropertyStreamConfiguration,
        scope,
        kAudioObjectPropertyElementWildcard,
    );

    let mut data_size = 0u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyDataSize(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut data_size),
        ))?;
    }

    let mut buffer = vec![0u8; data_size as usize];
    let mut size = data_size;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::new(buffer.as_mut_ptr()).unwrap().cast(),
        ))?;

        let list = &*(buffer.as_ptr() as *const AudioBufferList);
        for i in 0..list.mNumberBuffers {
            if list.mBuffers[i as usize].mNumberChannels > 0 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn device_name(device_id: DeviceId) -> Result<String> {
    let address = property_address(
        kAudioDevicePropertyDeviceNameCFString,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut name_ptr: *const CFString = null();
    let mut size = mem::size_of::<*const CFString>() as u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut name_ptr).cast(),
        ))?;
        let name_ptr = NonNull::new(name_ptr as *mut CFString).context("null device name")?;
        let name = CFRetained::from_raw(name_ptr);
        Ok(name.to_string())
    }
}

pub fn device_uid(device_id: DeviceId) -> Result<String> {
    let address = property_address(
        kAudioDevicePropertyDeviceUID,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut uid_ptr: *const CFString = null();
    let mut size = mem::size_of::<*const CFString>() as u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut uid_ptr).cast(),
        ))?;
        let uid_ptr = NonNull::new(uid_ptr as *mut CFString).context("null device uid")?;
        let uid = CFRetained::from_raw(uid_ptr);
        Ok(uid.to_string())
    }
}

pub fn device_sample_rate(device_id: DeviceId) -> Result<f64> {
    let address = property_address(
        kAudioDevicePropertyNominalSampleRate,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut rate = 0.0f64;
    let mut size = mem::size_of::<f64>() as u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut rate).cast(),
        ))?;
    }
    Ok(rate)
}

pub fn device_is_alive(device_id: DeviceId) -> Result<bool> {
    let address = property_address(
        kAudioDevicePropertyDeviceIsAlive,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut alive = 0u32;
    let mut size = mem::size_of::<u32>() as u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            device_id,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut alive).cast(),
        ))?;
    }
    Ok(alive != 0)
}

pub fn default_input_device_id() -> Result<DeviceId> {
    default_device_id(kAudioHardwarePropertyDefaultInputDevice)
}

pub fn default_output_device_id() -> Result<DeviceId> {
    default_device_id(kAudioHardwarePropertyDefaultOutputDevice)
}

fn default_device_id(selector: u32) -> Result<DeviceId> {
    let address = property_address(
        selector,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut device_id = 0u32;
    let mut size = mem::size_of::<DeviceId>() as u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            system_object(),
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut device_id).cast(),
        ))?;
    }
    if device_id == 0 {
        return Err(anyhow!("no default audio device"));
    }
    Ok(device_id)
}

pub fn device_id_for_uid(uid: &str) -> Result<DeviceId> {
    let cf_uid = CFString::from_str(uid);
    let uid_ref: *const CFString = CFRetained::as_ptr(&cf_uid).as_ptr();
    let address = property_address(
        kAudioHardwarePropertyTranslateUIDToDevice,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let qualifier_size = mem::size_of::<*const CFString>() as u32;

    let mut device_id = 0u32;
    let mut size = mem::size_of::<DeviceId>() as u32;
    unsafe {
        check_os_status(AudioObjectGetPropertyData(
            system_object(),
            NonNull::from(&address),
            qualifier_size,
            (&uid_ref as *const *const CFString).cast::<std::ffi::c_void>(),
            NonNull::from(&mut size),
            NonNull::from(&mut device_id).cast(),
        ))?;
    }
    if device_id == 0 {
        return Err(anyhow!("device uid not found: {uid}"));
    }
    Ok(device_id)
}

/// Add a property listener on the system object (device list, defaults).
///
/// # Safety
/// Same contract as [`AudioObjectAddPropertyListener`]: `listener` and
/// `client_data` must stay valid until removed.
pub unsafe fn add_system_property_listener(
    selector: u32,
    listener: AudioObjectPropertyListenerProc,
    client_data: *mut std::ffi::c_void,
) -> Result<()> {
    let address = property_address(
        selector,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    check_os_status(AudioObjectAddPropertyListener(
        system_object(),
        NonNull::from(&address),
        listener,
        client_data,
    ))
}

/// # Safety
/// Must pair a previous [`add_system_property_listener`] with the same args.
pub unsafe fn remove_system_property_listener(
    selector: u32,
    listener: AudioObjectPropertyListenerProc,
    client_data: *mut std::ffi::c_void,
) -> Result<()> {
    let address = property_address(
        selector,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    check_os_status(AudioObjectRemovePropertyListener(
        system_object(),
        NonNull::from(&address),
        listener,
        client_data,
    ))
}

/// Add a device-alive property listener.
///
/// # Safety
/// Same contract as [`AudioObjectAddPropertyListener`]: `listener` and
/// `client_data` must stay valid until removed.
pub unsafe fn add_device_alive_listener(
    device_id: DeviceId,
    listener: AudioObjectPropertyListenerProc,
    client_data: *mut std::ffi::c_void,
) -> Result<()> {
    let address = property_address(
        kAudioDevicePropertyDeviceIsAlive,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    check_os_status(AudioObjectAddPropertyListener(
        device_id,
        NonNull::from(&address),
        listener,
        client_data,
    ))
}

/// # Safety
/// Must pair a previous [`add_device_alive_listener`] with the same args.
pub unsafe fn remove_device_alive_listener(
    device_id: DeviceId,
    listener: AudioObjectPropertyListenerProc,
    client_data: *mut std::ffi::c_void,
) -> Result<()> {
    let address = property_address(
        kAudioDevicePropertyDeviceIsAlive,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    check_os_status(AudioObjectRemovePropertyListener(
        device_id,
        NonNull::from(&address),
        listener,
        client_data,
    ))
}

/// Add a nominal sample-rate property listener.
///
/// # Safety
/// Same contract as [`AudioObjectAddPropertyListener`]: `listener` and
/// `client_data` must stay valid until removed.
pub unsafe fn add_nominal_sample_rate_listener(
    device_id: DeviceId,
    listener: AudioObjectPropertyListenerProc,
    client_data: *mut std::ffi::c_void,
) -> Result<()> {
    let address = property_address(
        kAudioDevicePropertyNominalSampleRate,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    check_os_status(AudioObjectAddPropertyListener(
        device_id,
        NonNull::from(&address),
        listener,
        client_data,
    ))
}

/// # Safety
/// Must pair a previous [`add_nominal_sample_rate_listener`] with the same args.
pub unsafe fn remove_nominal_sample_rate_listener(
    device_id: DeviceId,
    listener: AudioObjectPropertyListenerProc,
    client_data: *mut std::ffi::c_void,
) -> Result<()> {
    let address = property_address(
        kAudioDevicePropertyNominalSampleRate,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    check_os_status(AudioObjectRemovePropertyListener(
        device_id,
        NonNull::from(&address),
        listener,
        client_data,
    ))
}
