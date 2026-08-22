//! Windows MMCSS (Multimedia Class Scheduler Service) thread priority elevation.

use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::{FALSE, HANDLE};
use windows_sys::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
};

/// Holds an MMCSS task handle for the current thread; reverts on drop.
/// Soft-fails: if elevation is unavailable, audio continues at normal priority.
pub struct MmcssGuard {
    handle: Option<HANDLE>,
}

impl MmcssGuard {
    pub fn enter() -> Self {
        // "Pro Audio" is the standard MMCSS task for low-latency audio threads.
        let task_name: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
        let mut task_index: u32 = 0;
        // SAFETY: `task_name` is a valid null-terminated UTF-16 string; `task_index`
        // is a writable out-param. A null return means MMCSS is unavailable.
        let handle =
            unsafe { AvSetMmThreadCharacteristicsW(task_name.as_ptr() as PCWSTR, &mut task_index) };
        if handle.is_null() {
            tracing::debug!("MMCSS Pro Audio elevation unavailable; continuing at normal priority");
            Self { handle: None }
        } else {
            Self {
                handle: Some(handle),
            }
        }
    }
}

impl Drop for MmcssGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // SAFETY: `handle` was returned by AvSetMmThreadCharacteristicsW on this thread.
            let ok = unsafe { AvRevertMmThreadCharacteristics(handle) };
            if ok == FALSE {
                tracing::debug!("AvRevertMmThreadCharacteristics failed");
            }
        }
    }
}
