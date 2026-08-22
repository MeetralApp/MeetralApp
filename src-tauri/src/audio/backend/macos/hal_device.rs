use anyhow::{anyhow, Context, Result};

use super::super::super::device::{AudioDeviceInfo, ResolvedDevice};
use super::hal_sys::{
    self, default_input_device_id, default_output_device_id, device_id_for_uid, device_name,
    device_supports_input, device_supports_output, device_uid, DeviceId,
};

#[derive(Debug, Clone)]
pub struct HalDevice {
    pub object_id: DeviceId,
    pub uid: String,
    pub name: String,
    pub direction: &'static str,
}

pub fn list_devices() -> Result<Vec<AudioDeviceInfo>> {
    let mut devices = Vec::new();
    for device_id in hal_sys::all_device_ids()? {
        let uid = device_uid(device_id).unwrap_or_else(|_| device_id.to_string());
        let name = device_name(device_id).unwrap_or_else(|_| format!("Device {device_id}"));

        if device_supports_input(device_id).unwrap_or(false) {
            devices.push(AudioDeviceInfo {
                id: uid.clone(),
                name: name.clone(),
                direction: "input".to_string(),
            });
        }
        if device_supports_output(device_id).unwrap_or(false) {
            devices.push(AudioDeviceInfo {
                id: uid,
                name,
                direction: "output".to_string(),
            });
        }
    }
    Ok(devices)
}

pub fn default_communications_device(direction: &str) -> Result<ResolvedDevice, String> {
    let device_id = if direction == "input" {
        default_input_device_id()
    } else {
        default_output_device_id()
    }
    .map_err(|e| e.to_string())?;

    let uid = device_uid(device_id).map_err(|e| e.to_string())?;
    let name = device_name(device_id).unwrap_or_else(|_| "System default".to_string());

    Ok(ResolvedDevice {
        id: uid,
        name,
        direction: if direction == "input" {
            "input"
        } else {
            "output"
        },
    })
}

pub fn resolve_hal_device(resolved: &ResolvedDevice) -> Result<HalDevice> {
    let object_id = if !resolved.id.is_empty() {
        device_id_for_uid(&resolved.id)
            .or_else(|_| resolve_device_id_from_stored_id(&resolved.id))
            .or_else(|_| find_device_by_name(resolved))
    } else {
        find_device_by_name(resolved)
    }
    .context(format!(
        "audio device not found: {} ({})",
        resolved.name, resolved.id
    ))?;

    let uid = device_uid(object_id).unwrap_or_else(|_| resolved.id.clone());
    let name = device_name(object_id).unwrap_or_else(|_| resolved.name.clone());

    Ok(HalDevice {
        object_id,
        uid,
        name,
        direction: resolved.direction,
    })
}

fn resolve_device_id_from_stored_id(id: &str) -> Result<DeviceId> {
    let device_id: DeviceId = id
        .parse()
        .map_err(|_| anyhow!("stored id is not a numeric device object id"))?;
    if device_id == 0 {
        return Err(anyhow!("invalid device object id"));
    }
    // Best-effort: confirm the object still exists in the current device list.
    if hal_sys::all_device_ids()?
        .into_iter()
        .any(|listed| listed == device_id)
    {
        Ok(device_id)
    } else {
        Err(anyhow!(
            "device object id {device_id} is not currently available"
        ))
    }
}

fn find_device_by_name(resolved: &ResolvedDevice) -> Result<DeviceId> {
    let needle = resolved.name.to_lowercase();
    if needle.is_empty() {
        return Err(anyhow!("empty device name"));
    }

    for device_id in hal_sys::all_device_ids()? {
        let supports = match resolved.direction {
            "input" => device_supports_input(device_id).unwrap_or(false),
            _ => device_supports_output(device_id).unwrap_or(false),
        };
        if !supports {
            continue;
        }
        let name = device_name(device_id).unwrap_or_default();
        if name.to_lowercase().contains(&needle) {
            return Ok(device_id);
        }
    }
    Err(anyhow!("no device matching {}", resolved.name))
}
