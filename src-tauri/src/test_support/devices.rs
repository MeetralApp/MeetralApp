use crate::audio::AudioDeviceInfo;

pub fn sample_devices() -> Vec<AudioDeviceInfo> {
    vec![
        AudioDeviceInfo {
            id: "mic".into(),
            name: "Mic".into(),
            direction: "input".into(),
        },
        AudioDeviceInfo {
            id: "teams".into(),
            name: "Teams".into(),
            direction: "output".into(),
        },
        AudioDeviceInfo {
            id: "cap".into(),
            name: "Capture".into(),
            direction: "input".into(),
        },
        AudioDeviceInfo {
            id: "hp".into(),
            name: "Headphones".into(),
            direction: "output".into(),
        },
    ]
}
