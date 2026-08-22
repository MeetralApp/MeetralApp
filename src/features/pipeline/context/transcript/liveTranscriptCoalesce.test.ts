import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  createTranscriptCoalesceScheduler,
  LIVE_TRANSCRIPT_FLUSH_MS,
} from "./liveTranscriptCoalesce";

describe("createTranscriptCoalesceScheduler", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("flushes a burst in one callback after LIVE_TRANSCRIPT_FLUSH_MS", () => {
    const flushed: string[][] = [];
    const scheduler = createTranscriptCoalesceScheduler<string>((batch) => {
      flushed.push([...batch]);
    });

    scheduler.enqueue("a");
    scheduler.enqueue("b");
    scheduler.enqueue("c");
    expect(flushed).toEqual([]);

    vi.advanceTimersByTime(LIVE_TRANSCRIPT_FLUSH_MS - 1);
    expect(flushed).toEqual([]);

    vi.advanceTimersByTime(1);
    expect(flushed).toEqual([["a", "b", "c"]]);
  });

  it("clear drops pending without flushing", () => {
    const flushed: string[][] = [];
    const scheduler = createTranscriptCoalesceScheduler<string>((batch) => {
      flushed.push([...batch]);
    });

    scheduler.enqueue("x");
    scheduler.clear();
    vi.advanceTimersByTime(LIVE_TRANSCRIPT_FLUSH_MS * 2);
    expect(flushed).toEqual([]);
  });
});
