use tokio::sync::mpsc;

/// Drain all pending messages from a receiver (drop them).
///
/// Works for bounded (`mpsc::Receiver`) PCM queues via `try_recv`.
pub fn drain_unbounded<T>(rx: &mut mpsc::Receiver<T>) -> usize {
    let mut count = 0usize;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn drain_empty_receiver() {
        let (_tx, mut rx) = mpsc::channel::<i32>(1);
        assert_eq!(drain_unbounded(&mut rx), 0);
    }

    #[tokio::test]
    async fn drain_non_empty_receiver() {
        let (tx, mut rx) = mpsc::channel::<i32>(4);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        assert_eq!(drain_unbounded(&mut rx), 2);
        assert_eq!(drain_unbounded(&mut rx), 0);
    }
}
