//! `IMMNotificationClient`-based endpoint change notifications.
//!
//! A dedicated COM (MTA) thread registers the callback with the MMDevice
//! enumerator and forwards every catalog-affecting event into a channel that
//! wakes the engine watchdog. The events are deliberately payload-free: they
//! arrive in bursts during endpoint churn (Bluetooth profile switches, virtual
//! driver engine restarts), so the watchdog re-enumerates and lets the catalog
//! stability window decide what to do.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
    MMDeviceEnumerator, DEVICE_STATE,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows_core::{implement, PCWSTR};

use super::super::super::device_monitor::{DeviceChangeEvent, DeviceChangeSender};

pub struct EndpointMonitorGuard {
    cancel: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for EndpointMonitorGuard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[implement(IMMNotificationClient)]
struct EndpointNotificationClient {
    tx: DeviceChangeSender,
}

impl EndpointNotificationClient {
    fn notify(&self, kind: &str, id: &PCWSTR) {
        tracing::debug!(
            kind,
            endpoint = endpoint_id(id),
            "audio endpoint notification"
        );
        let _ = self.tx.send(DeviceChangeEvent);
    }
}

impl IMMNotificationClient_Impl for EndpointNotificationClient_Impl {
    fn OnDeviceAdded(&self, id: &PCWSTR) -> windows_core::Result<()> {
        self.notify("added", id);
        Ok(())
    }

    fn OnDeviceRemoved(&self, id: &PCWSTR) -> windows_core::Result<()> {
        self.notify("removed", id);
        Ok(())
    }

    fn OnDeviceStateChanged(&self, id: &PCWSTR, _state: DEVICE_STATE) -> windows_core::Result<()> {
        self.notify("state-changed", id);
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        _flow: EDataFlow,
        _role: ERole,
        id: &PCWSTR,
    ) -> windows_core::Result<()> {
        // Empty device roles resolve to the system default, so default changes
        // can rewire the catalog we care about.
        self.notify("default-changed", id);
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> windows_core::Result<()> {
        // Too noisy (fires on volume changes); catalog diffs are detected by
        // re-enumeration after the events above.
        Ok(())
    }
}

fn endpoint_id(id: &PCWSTR) -> String {
    if id.0.is_null() {
        return String::new();
    }
    // SAFETY: COM guarantees the pointer is valid for the duration of the callback.
    unsafe { id.to_string().unwrap_or_default() }
}

pub fn start_endpoint_monitor(tx: DeviceChangeSender) -> Result<EndpointMonitorGuard> {
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_thread = cancel.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<Result<()>>(1);

    let thread = thread::Builder::new()
        .name("audio-endpoint-notify".to_string())
        .spawn(move || {
            if let Err(error) = run_endpoint_monitor(tx, cancel_thread, ready_tx) {
                tracing::warn!("endpoint notification monitor stopped: {error:#}");
            }
        })
        .context("spawn endpoint notification thread")?;

    // Surface registration failures synchronously so the caller can fall back
    // to polling immediately.
    match ready_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(EndpointMonitorGuard {
            cancel,
            thread: Some(thread),
        }),
        Ok(Err(error)) => {
            cancel.store(true, Ordering::SeqCst);
            let _ = thread.join();
            Err(error)
        }
        Err(_) => {
            cancel.store(true, Ordering::SeqCst);
            let _ = thread.join();
            anyhow::bail!("endpoint notification registration timed out")
        }
    }
}

fn run_endpoint_monitor(
    tx: DeviceChangeSender,
    cancel: Arc<AtomicBool>,
    ready: std::sync::mpsc::SyncSender<Result<()>>,
) -> Result<()> {
    super::wasapi_util::init_com()?;

    let registered = (|| -> Result<(IMMDeviceEnumerator, IMMNotificationClient)> {
        // SAFETY: standard single-threaded COM usage on our own dedicated
        // thread; the enumerator and client outlive the registration.
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
                .context("create MMDeviceEnumerator")?;
        let client: IMMNotificationClient = EndpointNotificationClient { tx }.into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&client) }
            .context("RegisterEndpointNotificationCallback")?;
        Ok((enumerator, client))
    })();

    let (enumerator, client) = match registered {
        Ok(pair) => {
            let _ = ready.send(Ok(()));
            pair
        }
        Err(error) => {
            let _ = ready.send(Err(error.context("endpoint notification setup")));
            return Ok(());
        }
    };

    while !cancel.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }

    // COM rule: never unregister from inside a callback — this runs on the
    // monitor thread after the cancel flag is set.
    // SAFETY: unregisters the exact client registered above; callbacks stop
    // once this returns, and `client`/`enumerator` are dropped right after.
    unsafe {
        let _ = enumerator.UnregisterEndpointNotificationCallback(&client);
    }
    Ok(())
}
