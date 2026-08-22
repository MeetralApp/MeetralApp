//! Bounded playback PCM channels with overflow accounting.

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

/// ~hundreds of ms of TTS/playback chunks at typical synthesis rates.
pub const PLAYBACK_PCM_CHANNEL_DEPTH: usize = 24;

/// Try to enqueue `item` on a bounded channel. On full, drop the **incoming**
/// item (newest) and increment `drops` — same backpressure shape as capture
/// `try_send` (Sender-only sites cannot drain oldest without a co-located rx).
///
/// Returns `true` if enqueued, `false` if closed or dropped due to full.
pub fn try_send_pcm_bounded<T>(tx: &mpsc::Sender<T>, item: T, drops: &AtomicU64) -> bool {
    match tx.try_send(item) {
        Ok(()) => true,
        Err(mpsc::error::TrySendError::Closed(_)) => false,
        Err(mpsc::error::TrySendError::Full(_)) => {
            drops.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

/// When both ends are available: on Full, drop oldest then retry with newest.
pub fn try_send_pcm_drop_oldest<T>(
    tx: &mpsc::Sender<T>,
    rx: &mut mpsc::Receiver<T>,
    item: T,
    drops: &AtomicU64,
) -> bool {
    match tx.try_send(item) {
        Ok(()) => true,
        Err(mpsc::error::TrySendError::Closed(_)) => false,
        Err(mpsc::error::TrySendError::Full(item)) => {
            let _ = rx.try_recv();
            drops.fetch_add(1, Ordering::Relaxed);
            match tx.try_send(item) {
                Ok(()) => true,
                Err(mpsc::error::TrySendError::Closed(_)) => false,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    drops.fetch_add(1, Ordering::Relaxed);
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_drop_newest_increments_counter() {
        let (tx, mut rx) = mpsc::channel::<u32>(1);
        let drops = AtomicU64::new(0);
        assert!(try_send_pcm_bounded(&tx, 1, &drops));
        assert!(!try_send_pcm_bounded(&tx, 2, &drops));
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(rx.recv().await, Some(1));
    }

    #[tokio::test]
    async fn drop_oldest_retains_newest_and_increments_counter() {
        let (tx, mut rx) = mpsc::channel::<u32>(2);
        let drops = AtomicU64::new(0);

        assert!(try_send_pcm_drop_oldest(&tx, &mut rx, 1, &drops));
        assert!(try_send_pcm_drop_oldest(&tx, &mut rx, 2, &drops));
        assert!(try_send_pcm_drop_oldest(&tx, &mut rx, 3, &drops));
        assert_eq!(drops.load(Ordering::Relaxed), 1);

        assert_eq!(rx.try_recv().ok(), Some(2));
        assert_eq!(rx.try_recv().ok(), Some(3));
        assert!(rx.try_recv().is_err());
    }
}
