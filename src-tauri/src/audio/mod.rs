pub mod backend;
pub mod capture;
pub mod device;
pub mod device_monitor;
pub mod ducking_mix;
pub mod pcm_channel;
pub mod pcm_crossfade;
pub mod playback;
pub mod playback_buffer;
pub mod resampler;
pub mod runtime;

#[cfg(target_os = "macos")]
pub use backend::macos::start_direct_passthrough;
#[cfg(windows)]
pub use backend::windows::start_direct_passthrough;
pub use capture::{
    monotonic_ms, start_meeting_capture_for_config, start_user_mic_capture, AudioFaultEvent,
    AudioFaultSender, CaptureHandle, CaptureHeartbeat, CaptureSender, CAPTURE_CHANNEL_DEPTH,
};
pub use device::{
    backfill_device_ids, list_devices, list_devices_async, resolve_role_device,
    validate_audio_setup, validate_for_direct_inbound, validate_for_direct_outbound,
    validate_for_start_inbound, validate_for_start_outbound, AudioDeviceInfo, AudioRole,
    AudioSetupValidation, ResolvedDevice, RoleValidation,
};
pub use device_monitor::{start_device_change_monitor, DeviceChangeEvent, DeviceChangeSender};
pub use pcm_channel::{try_send_pcm_bounded, try_send_pcm_drop_oldest, PLAYBACK_PCM_CHANNEL_DEPTH};
pub use pcm_crossfade::{
    spawn_pcm24k_adapter, PcmChunkBoundary, PcmCrossfadeMixer, PlaybackCrossfadeOptions,
    PlaybackPcmChunk,
};
pub use playback::{start_playback, start_playback_for_role, PlaybackHandle};
pub use playback_buffer::{PlaybackBufferConfig, PlaybackFillStatus, PlaybackRingBuffer};
pub use runtime::{
    gate_pcm, gate_pcm_in_place, send_passthrough, shared_playback_device,
    spawn_bridge_audio_drain, spawn_pipeline_audio, AudioModeHandle, MicMuteHandle,
    SharedPlaybackDevice,
};
