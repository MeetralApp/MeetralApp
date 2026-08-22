use serde::Serialize;

use crate::config::{AppConfig, DeviceRef};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioRole {
    UserMic,
    TeamsMicFeed,
    MeetingCapture,
    LocalPlayback,
}

impl AudioRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::UserMic => "Your microphone",
            Self::TeamsMicFeed => "Teams mic feed",
            Self::MeetingCapture => "Meeting capture",
            Self::LocalPlayback => "Local playback",
        }
    }

    pub fn direction(self) -> &'static str {
        match self {
            Self::UserMic | Self::MeetingCapture => "input",
            Self::TeamsMicFeed | Self::LocalPlayback => "output",
        }
    }

    pub fn device_ref(self, config: &AppConfig) -> &DeviceRef {
        match self {
            Self::UserMic => &config.user_mic,
            Self::TeamsMicFeed => &config.teams_mic_feed,
            Self::MeetingCapture => &config.meeting_capture,
            Self::LocalPlayback => &config.local_playback,
        }
    }

    pub fn device_ref_mut(self, config: &mut AppConfig) -> &mut DeviceRef {
        match self {
            Self::UserMic => &mut config.user_mic,
            Self::TeamsMicFeed => &mut config.teams_mic_feed,
            Self::MeetingCapture => &mut config.meeting_capture,
            Self::LocalPlayback => &mut config.local_playback,
        }
    }

    pub fn allows_system_default(self) -> bool {
        matches!(self, Self::UserMic | Self::LocalPlayback)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedDevice {
    pub id: String,
    pub name: String,
    pub direction: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoleValidation {
    pub role: AudioRole,
    pub required: bool,
    pub configured: bool,
    pub resolved: bool,
    pub resolved_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSetupValidation {
    pub outbound_ready: bool,
    pub inbound_ready: bool,
    pub roles: Vec<RoleValidation>,
}

#[cfg(windows)]
pub fn list_devices() -> anyhow::Result<Vec<AudioDeviceInfo>> {
    super::backend::windows::device::list_devices()
}

#[cfg(target_os = "macos")]
pub fn list_devices() -> anyhow::Result<Vec<AudioDeviceInfo>> {
    super::backend::macos::hal_device::list_devices()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn list_devices() -> anyhow::Result<Vec<AudioDeviceInfo>> {
    anyhow::bail!("Audio device listing is not supported on this platform")
}

/// Enumerate audio devices off the async runtime so the WASAPI/COM blocking call
/// never stalls a tokio worker (and is never run while holding the engine lock).
pub async fn list_devices_async() -> anyhow::Result<Vec<AudioDeviceInfo>> {
    tokio::task::spawn_blocking(list_devices)
        .await
        .map_err(|e| anyhow::anyhow!("list_devices task failed: {e}"))?
}

#[cfg(windows)]
pub fn default_communications_device(direction: &str) -> Result<ResolvedDevice, String> {
    super::backend::windows::device::default_communications_device(direction)
}

#[cfg(target_os = "macos")]
pub fn default_communications_device(direction: &str) -> Result<ResolvedDevice, String> {
    super::backend::macos::hal_device::default_communications_device(direction)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn default_communications_device(_direction: &str) -> Result<ResolvedDevice, String> {
    Err("default audio device is not supported on this platform".into())
}

pub fn resolve_device_by_id_or_name(
    id: &str,
    name: &str,
    direction: &str,
    devices: &[AudioDeviceInfo],
) -> Result<ResolvedDevice, String> {
    let id = id.trim();
    let name = name.trim();

    if !id.is_empty() {
        if let Some(device) = devices
            .iter()
            .find(|d| d.direction == direction && d.id == id)
        {
            return Ok(ResolvedDevice {
                id: device.id.clone(),
                name: device.name.clone(),
                direction: if direction == "input" {
                    "input"
                } else {
                    "output"
                },
            });
        }
    }

    if !name.is_empty() {
        let needle = name.to_lowercase();
        if let Some(device) = devices
            .iter()
            .find(|d| d.direction == direction && d.name.to_lowercase().contains(&needle))
        {
            return Ok(ResolvedDevice {
                id: device.id.clone(),
                name: device.name.clone(),
                direction: if direction == "input" {
                    "input"
                } else {
                    "output"
                },
            });
        }
    }

    let detail = if !id.is_empty() && !name.is_empty() {
        format!("id={id}, name={name}")
    } else if !id.is_empty() {
        format!("id={id}")
    } else if !name.is_empty() {
        format!("name={name}")
    } else {
        "not configured".to_string()
    };

    Err(format!("device not found ({detail})"))
}

pub fn resolve_role_device(
    role: AudioRole,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> Result<ResolvedDevice, String> {
    let device_ref = role.device_ref(config);
    let direction = role.direction();

    if device_ref.is_empty() {
        if role.allows_system_default() {
            return default_communications_device(direction);
        }
        return Err(format!("{}: not configured", role.label()));
    }

    resolve_device_by_id_or_name(&device_ref.id, &device_ref.name, direction, devices)
        .map_err(|e| format!("{}: {e}", role.label()))
}

pub fn validate_role(
    role: AudioRole,
    required: bool,
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> RoleValidation {
    let device_ref = role.device_ref(config);
    let configured = device_ref.is_configured() || (role.allows_system_default() && required);

    if required && device_ref.is_empty() && !role.allows_system_default() {
        return RoleValidation {
            role,
            required,
            configured: false,
            resolved: false,
            resolved_name: None,
            error: Some(format!("{}: not configured", role.label())),
        };
    }

    if !required && device_ref.is_empty() && !role.allows_system_default() {
        return RoleValidation {
            role,
            required,
            configured: false,
            resolved: true,
            resolved_name: None,
            error: None,
        };
    }

    match resolve_role_device(role, config, devices) {
        Ok(resolved) => RoleValidation {
            role,
            required,
            configured,
            resolved: true,
            resolved_name: Some(resolved.name),
            error: None,
        },
        Err(error) => RoleValidation {
            role,
            required,
            configured,
            resolved: false,
            resolved_name: None,
            error: Some(error),
        },
    }
}

pub fn validate_audio_setup(
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> AudioSetupValidation {
    let outbound_playback_required = config.outbound_mode.needs_playback();
    let inbound_playback_required = config.inbound_mode.needs_playback();

    let roles = vec![
        validate_role(AudioRole::UserMic, true, config, devices),
        validate_role(
            AudioRole::TeamsMicFeed,
            outbound_playback_required,
            config,
            devices,
        ),
        validate_role(AudioRole::MeetingCapture, true, config, devices),
        validate_role(
            AudioRole::LocalPlayback,
            inbound_playback_required,
            config,
            devices,
        ),
    ];

    let outbound_ready = roles
        .iter()
        .filter(|r| matches!(r.role, AudioRole::UserMic | AudioRole::TeamsMicFeed) && r.required)
        .all(|r| r.resolved);
    let inbound_ready = roles
        .iter()
        .filter(|r| {
            matches!(r.role, AudioRole::MeetingCapture | AudioRole::LocalPlayback) && r.required
        })
        .all(|r| r.resolved);

    AudioSetupValidation {
        outbound_ready,
        inbound_ready,
        roles,
    }
}

pub fn validate_for_start_outbound(
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> Result<(), String> {
    config.validate_for_start()?;
    let validation = validate_audio_setup(config, devices);
    for role in validation.roles {
        if !role.required {
            continue;
        }
        if matches!(role.role, AudioRole::UserMic | AudioRole::TeamsMicFeed) && !role.resolved {
            return Err(role
                .error
                .unwrap_or_else(|| format!("{}: not ready", role.role.label())));
        }
    }
    Ok(())
}

pub fn validate_for_start_inbound(
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> Result<(), String> {
    config.validate_for_start()?;
    let validation = validate_audio_setup(config, devices);
    for role in validation.roles {
        if !role.required {
            continue;
        }
        if matches!(
            role.role,
            AudioRole::MeetingCapture | AudioRole::LocalPlayback
        ) && !role.resolved
        {
            return Err(role
                .error
                .unwrap_or_else(|| format!("{}: not ready", role.role.label())));
        }
    }
    Ok(())
}

pub fn validate_for_direct_outbound(
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> Result<(), String> {
    let mut probe = config.clone();
    probe.outbound_mode = crate::config::PipelineOutputMode::Translated;
    let validation = validate_audio_setup(&probe, devices);
    for role in validation.roles {
        if !matches!(role.role, AudioRole::UserMic | AudioRole::TeamsMicFeed) {
            continue;
        }
        if !role.resolved {
            return Err(role
                .error
                .unwrap_or_else(|| format!("{}: not ready", role.role.label())));
        }
    }
    Ok(())
}

pub fn validate_for_direct_inbound(
    config: &AppConfig,
    devices: &[AudioDeviceInfo],
) -> Result<(), String> {
    let mut probe = config.clone();
    probe.inbound_mode = crate::config::PipelineOutputMode::Translated;
    let validation = validate_audio_setup(&probe, devices);
    for role in validation.roles {
        if !matches!(
            role.role,
            AudioRole::MeetingCapture | AudioRole::LocalPlayback
        ) {
            continue;
        }
        if !role.resolved {
            return Err(role
                .error
                .unwrap_or_else(|| format!("{}: not ready", role.role.label())));
        }
    }
    Ok(())
}

pub fn backfill_device_ids(config: &mut AppConfig, devices: &[AudioDeviceInfo]) -> bool {
    let mut changed = false;
    for role in [
        AudioRole::UserMic,
        AudioRole::TeamsMicFeed,
        AudioRole::MeetingCapture,
        AudioRole::LocalPlayback,
    ] {
        let device_ref = role.device_ref_mut(config);
        if !device_ref.id.is_empty() || device_ref.name.trim().is_empty() {
            continue;
        }
        if let Ok(resolved) =
            resolve_device_by_id_or_name("", &device_ref.name, role.direction(), devices)
        {
            device_ref.id = resolved.id;
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, PipelineOutputMode};

    fn device(id: &str, name: &str, direction: &str) -> AudioDeviceInfo {
        AudioDeviceInfo {
            id: id.to_string(),
            name: name.to_string(),
            direction: direction.to_string(),
        }
    }

    fn config_with(
        user_mic: DeviceRef,
        teams_mic_feed: DeviceRef,
        meeting_capture: DeviceRef,
        local_playback: DeviceRef,
    ) -> AppConfig {
        AppConfig {
            user_mic,
            teams_mic_feed,
            meeting_capture,
            local_playback,
            ..AppConfig::default()
        }
    }

    #[test]
    fn resolves_by_exact_id() {
        let devices = vec![device("id-1", "Virtual Out B2", "input")];
        let resolved =
            resolve_device_by_id_or_name("id-1", "", "input", &devices).expect("resolve");
        assert_eq!(resolved.id, "id-1");
    }

    #[test]
    fn resolves_by_name_substring() {
        let devices = vec![device("id-aux", "My AUX Input Device", "output")];
        let resolved =
            resolve_device_by_id_or_name("", "aux input", "output", &devices).expect("resolve");
        assert_eq!(resolved.id, "id-aux");
    }

    #[test]
    fn missing_device_returns_error() {
        let err = resolve_device_by_id_or_name("", "missing", "input", &[]).unwrap_err();
        assert!(err.contains("device not found"));
    }

    #[test]
    fn outbound_translated_requires_teams_mic_feed() {
        let config = config_with(
            DeviceRef::empty(),
            DeviceRef::empty(),
            DeviceRef {
                id: "cap".into(),
                name: "Capture".into(),
            },
            DeviceRef::empty(),
        );
        let devices = vec![device("cap", "Capture", "input")];
        let validation = validate_audio_setup(&config, &devices);
        assert!(!validation.outbound_ready);
        let teams = validation
            .roles
            .iter()
            .find(|r| r.role == AudioRole::TeamsMicFeed)
            .expect("teams role");
        assert!(teams.required);
        assert!(!teams.resolved);
    }

    #[test]
    fn outbound_text_only_skips_teams_mic_feed() {
        let devices = vec![device("mic", "Mic", "input")];
        let mut config = config_with(
            DeviceRef {
                id: "mic".into(),
                name: "Mic".into(),
            },
            DeviceRef::empty(),
            DeviceRef::empty(),
            DeviceRef::empty(),
        );
        config.outbound_mode = PipelineOutputMode::TextOnly;
        let validation = validate_audio_setup(&config, &devices);
        let teams = validation
            .roles
            .iter()
            .find(|r| r.role == AudioRole::TeamsMicFeed)
            .expect("teams role");
        assert!(!teams.required);
        assert!(validation.outbound_ready);
    }

    #[test]
    fn backfill_sets_id_from_name() {
        let devices = vec![device("resolved-id", "Meeting Capture Device", "input")];
        let mut config = config_with(
            DeviceRef::empty(),
            DeviceRef::empty(),
            DeviceRef {
                id: String::new(),
                name: "Meeting Capture".into(),
            },
            DeviceRef::empty(),
        );
        assert!(backfill_device_ids(&mut config, &devices));
        assert_eq!(config.meeting_capture.id, "resolved-id");
    }

    #[test]
    fn direct_outbound_does_not_require_api_key() {
        let devices = vec![
            device("mic", "Mic", "input"),
            device("aux", "AUX Input", "output"),
        ];
        let config = config_with(
            DeviceRef {
                id: "mic".into(),
                name: "Mic".into(),
            },
            DeviceRef {
                id: "aux".into(),
                name: "AUX Input".into(),
            },
            DeviceRef::empty(),
            DeviceRef::empty(),
        );
        assert!(validate_for_direct_outbound(&config, &devices).is_ok());
    }
}
