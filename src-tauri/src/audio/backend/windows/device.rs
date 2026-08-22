use anyhow::anyhow;
use wasapi::{DeviceEnumerator, Direction, Role};

use super::super::super::device::{AudioDeviceInfo, ResolvedDevice};
use super::wasapi_util::device_open_lock;

pub fn list_devices() -> anyhow::Result<Vec<AudioDeviceInfo>> {
    let enumerator = DeviceEnumerator::new()?;
    let mut devices = Vec::new();

    for direction in [Direction::Capture, Direction::Render] {
        let dir_label = if direction == Direction::Capture {
            "input"
        } else {
            "output"
        };
        let collection = enumerator.get_device_collection(&direction)?;
        let count = collection.get_nbr_devices()?;
        for idx in 0..count {
            let device = collection.get_device_at_index(idx)?;
            let id = device
                .get_id()
                .unwrap_or_else(|_| format!("{dir_label}:{idx}"));
            let name = device
                .get_friendlyname()
                .unwrap_or_else(|_| format!("{dir_label}-{idx}"));
            devices.push(AudioDeviceInfo {
                id,
                name,
                direction: dir_label.to_string(),
            });
        }
    }

    Ok(devices)
}

pub fn default_communications_device(direction: &str) -> Result<ResolvedDevice, String> {
    let wasapi_dir = if direction == "input" {
        Direction::Capture
    } else {
        Direction::Render
    };

    let enumerator = DeviceEnumerator::new().map_err(|e| e.to_string())?;
    let device = enumerator
        .get_default_device_for_role(&wasapi_dir, &Role::Communications)
        .or_else(|_| enumerator.get_default_device(&wasapi_dir))
        .map_err(|e| format!("no default {direction} device: {e}"))?;

    Ok(ResolvedDevice {
        id: device.get_id().map_err(|e| e.to_string())?,
        name: device
            .get_friendlyname()
            .unwrap_or_else(|_| "System default".to_string()),
        direction: if direction == "input" {
            "input"
        } else {
            "output"
        },
    })
}

pub fn open_wasapi_device(resolved: &ResolvedDevice) -> anyhow::Result<wasapi::Device> {
    let _open_guard = device_open_lock();

    let direction = if resolved.direction == "input" {
        Direction::Capture
    } else {
        Direction::Render
    };

    let enumerator = DeviceEnumerator::new()?;
    let collection = enumerator.get_device_collection(&direction)?;
    let count = collection.get_nbr_devices()?;
    let needle = resolved.name.to_lowercase();

    if !resolved.id.is_empty() {
        for idx in 0..count {
            let device = collection.get_device_at_index(idx)?;
            let id = device.get_id().unwrap_or_default();
            if id == resolved.id {
                return Ok(device);
            }
        }
    }

    if !needle.is_empty() {
        for idx in 0..count {
            let device = collection.get_device_at_index(idx)?;
            let name = device.get_friendlyname().unwrap_or_default();
            if name.to_lowercase().contains(&needle) {
                return Ok(device);
            }
        }
    }

    Err(anyhow!(
        "audio device not found: {} ({})",
        resolved.name,
        resolved.id
    ))
}
