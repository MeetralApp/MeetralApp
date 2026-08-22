//! OS-level audio endpoint change notifications.
//!
//! The event is payload-free on purpose: endpoint notifications arrive in
//! bursts during driver churn (Bluetooth HFP/A2DP profile switches, virtual
//! audio driver engine restarts), so the only sane reaction is to wake the
//! watchdog, re-enumerate, and let the catalog stability window decide.

use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceChangeEvent;

pub type DeviceChangeSender = mpsc::UnboundedSender<DeviceChangeEvent>;
pub type DeviceChangeReceiver = mpsc::UnboundedReceiver<DeviceChangeEvent>;

/// Register the platform endpoint listener, if available. Returns a guard that
/// keeps the listener registered; dropping it unregisters. `None` means the
/// platform (or registration failure) leaves the watchdog on polling only.
#[cfg(windows)]
pub fn start_device_change_monitor(
    tx: DeviceChangeSender,
) -> Option<super::backend::windows::endpoint_notify::EndpointMonitorGuard> {
    match super::backend::windows::endpoint_notify::start_endpoint_monitor(tx) {
        Ok(guard) => Some(guard),
        Err(error) => {
            tracing::warn!("endpoint notification registration failed: {error:#}");
            None
        }
    }
}

#[cfg(target_os = "macos")]
pub fn start_device_change_monitor(
    tx: DeviceChangeSender,
) -> Option<super::backend::macos::listener::DeviceListListener> {
    match super::backend::macos::listener::DeviceListListener::register(tx) {
        Ok(guard) => Some(guard),
        Err(error) => {
            tracing::warn!("device list listener registration failed: {error:#}");
            None
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn start_device_change_monitor(_tx: DeviceChangeSender) -> Option<()> {
    None
}
