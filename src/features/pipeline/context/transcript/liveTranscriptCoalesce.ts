/** Cap React commits during dense STT interim bursts (parity with Overlay). */
export const LIVE_TRANSCRIPT_FLUSH_MS = 80;

export type TranscriptCoalesceScheduler<T> = {
  enqueue: (item: T) => void;
  clear: () => void;
  dispose: () => void;
};

/**
* Throttles bursts into one flush every `flushMs`.
* Used by Live TranscriptProvider; unit-tested with fake timers.
*/
export function createTranscriptCoalesceScheduler<T>(
  flush: (batch: T[]) => void,
  flushMs: number = LIVE_TRANSCRIPT_FLUSH_MS,
  setTimer: (fn: () => void, ms: number) => ReturnType<typeof setTimeout> = setTimeout,
  clearTimer: (id: ReturnType<typeof setTimeout>) => void = clearTimeout,
): TranscriptCoalesceScheduler<T> {
  let pending: T[] = [];
  let scheduled = false;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const runFlush = () => {
    scheduled = false;
    timer = null;
    const batch = pending;
    pending = [];
    if (batch.length > 0) {
      flush(batch);
    }
  };

  return {
    enqueue(item: T) {
      pending.push(item);
      if (scheduled) return;
      scheduled = true;
      timer = setTimer(runFlush, flushMs);
    },
    clear() {
      scheduled = false;
      if (timer != null) {
        clearTimer(timer);
        timer = null;
      }
      pending = [];
    },
    dispose() {
      this.clear();
    },
  };
}
