import { useCallback, useEffect, useRef, useState } from "react";
import { Pause, Play, RotateCcw, RotateCw } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import LiveNotice from "@/shared/components/LiveNotice";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import {
  decodeMeetingAudioWindow,
  verifyMeetingAudio,
  type MeetingAudioChunk,
  type MeetingAudioSource,
  type MissingAudioSpan,
} from "@/features/meeting/library/lib/meetingApi";
import {
  findNearestSegmentAtTime,
  formatSegmentTimestamp,
  type TimelineDirectionFilter,
} from "../lib/timelineTranscript";
import AudioSourceFilterMenu from "./AudioSourceFilterMenu";

const WINDOW_MS = 1500;
const PREFETCH_LEAD_MS = 400;
const HIGHLIGHT_TOLERANCE_MS = 500;
const SKIP_MS = 5_000;
const SPEED_STEPS = [1, 1.25, 1.5, 2] as const;
type PlaybackSpeed = (typeof SPEED_STEPS)[number];

/** Floating chrome over the transcript pane (not a docked footer). */
const floatingPlayerShellClass = cn(
  "flex flex-col overflow-hidden rounded-lg border border-border/80 bg-secondary",
  "shadow-[var(--pane-elevated)] ring-1 ring-foreground/10",
);

interface Props {
  meetingId: string;
  segments: TranscriptSegment[];
  /** Synced from transcript filter: All→room, You→you, Meeting→meeting. */
  audioSource: MeetingAudioSource;
  filter: TimelineDirectionFilter;
  onFilterChange: (next: TimelineDirectionFilter) => void;
  onFocusSegment: (segment: TranscriptSegment) => void;
  seekToMs: number | null;
  onSeekConsumed: () => void;
}

function chunkDurationMs(chunks: MeetingAudioChunk[]): number {
  let max = 0;
  for (const c of chunks) {
    max = Math.max(max, c.startedAtMs + c.durationMs);
  }
  return max;
}

function segmentDurationMs(segments: TranscriptSegment[]): number {
  let max = 0;
  for (const s of segments) {
    max = Math.max(max, s.endedAtMs ?? s.startedAtMs);
  }
  return max;
}

function formatSpeedLabel(rate: PlaybackSpeed): string {
  if (rate === 1) return "1×";
  if (rate === 1.25) return "1.25×";
  if (rate === 1.5) return "1.5×";
  return "2×";
}

function incompleteWarning(missingCount: number, recordedCount: number): string {
  if (missingCount <= 0) return "";
  if (missingCount >= recordedCount) {
    return "Recording files are missing from disk.";
  }
  return `${missingCount} of ${recordedCount} audio files are missing. Playback stops at gaps.`;
}

/** True when [fromMs, fromMs+dur) overlaps a missing chunk for the active source. */
function hitsMissingSpan(
  fromMs: number,
  durationMs: number,
  source: MeetingAudioSource,
  missing: MissingAudioSpan[],
): boolean {
  if (!missing.length) return false;
  const endMs = fromMs + durationMs;
  for (const span of missing) {
    if (source === "you" && span.direction !== "outbound") continue;
    if (source === "meeting" && span.direction !== "inbound") continue;
    const sEnd = span.startedAtMs + span.durationMs;
    if (sEnd > fromMs && span.startedAtMs < endMs) return true;
  }
  return false;
}

function pcmToAudioBuffer(
  ctx: AudioContext,
  samples: number[],
  srcRate: number,
): AudioBuffer {
  if (Math.abs(ctx.sampleRate - srcRate) < 1) {
    const buffer = ctx.createBuffer(1, samples.length, ctx.sampleRate);
    buffer.copyToChannel(Float32Array.from(samples), 0);
    return buffer;
  }
  const ratio = ctx.sampleRate / srcRate;
  const outLen = Math.max(1, Math.floor(samples.length * ratio));
  const buffer = ctx.createBuffer(1, outLen, ctx.sampleRate);
  const out = buffer.getChannelData(0);
  for (let i = 0; i < outLen; i++) {
    const srcPos = i / ratio;
    const i0 = Math.floor(srcPos);
    const i1 = Math.min(i0 + 1, samples.length - 1);
    const frac = srcPos - i0;
    out[i] = samples[i0]! * (1 - frac) + samples[i1]! * frac;
  }
  return buffer;
}

export default function MeetingAudioPlayer({
  meetingId,
  segments,
  audioSource,
  filter,
  onFilterChange,
  onFocusSegment,
  seekToMs,
  onSeekConsumed,
}: Props) {
  const [hasAudio, setHasAudio] = useState<boolean | null>(null);
  const [chunks, setChunks] = useState<MeetingAudioChunk[]>([]);
  const [playing, setPlaying] = useState(false);
  const [tMs, setTMs] = useState(0);
  const [speed, setSpeed] = useState<PlaybackSpeed>(1);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [audioWarning, setAudioWarning] = useState<string | null>(null);
  const [recordingIncomplete, setRecordingIncomplete] = useState(false);

  const audioCtxRef = useRef<AudioContext | null>(null);
  const playingRef = useRef(false);
  const tMsRef = useRef(0);
  const sourceRef = useRef<MeetingAudioSource>(audioSource);
  const speedRef = useRef<PlaybackSpeed>(1);
  const durationRef = useRef(0);
  const scheduledUntilRef = useRef(0);
  const nextStartAtRef = useRef(0);
  const sourcesRef = useRef<AudioBufferSourceNode[]>([]);
  const rafRef = useRef<number | null>(null);
  const fetchGenRef = useRef(0);
  const scheduleLockRef = useRef(false);
  const lastFocusIdRef = useRef<string | null>(null);
  const playAnchorWallRef = useRef(0);
  const playAnchorTRef = useRef(0);
  const segmentsRef = useRef(segments);
  const onFocusRef = useRef(onFocusSegment);
  const missingSpansRef = useRef<MissingAudioSpan[]>([]);

  segmentsRef.current = segments;
  onFocusRef.current = onFocusSegment;
  sourceRef.current = audioSource;
  speedRef.current = speed;

  // Incomplete: stick to playable file extent. Complete: align with transcript.
  const durationMs = Math.max(
    chunkDurationMs(chunks),
    recordingIncomplete ? 0 : segmentDurationMs(segments),
    1,
  );
  durationRef.current = durationMs;

  useEffect(() => {
    let cancelled = false;
    setHasAudio(null);
    setLoadError(null);
    setAudioWarning(null);
    setRecordingIncomplete(false);
    setChunks([]);
    missingSpansRef.current = [];
    setPlaying(false);
    playingRef.current = false;
    setTMs(0);
    tMsRef.current = 0;
    scheduledUntilRef.current = 0;

    void (async () => {
      try {
        const verified = await verifyMeetingAudio(meetingId);
        if (cancelled) return;
        missingSpansRef.current = verified.missing;
        setChunks(verified.chunks);
        const incomplete = verified.missingCount > 0;
        setRecordingIncomplete(incomplete);
        setHasAudio(verified.chunks.length > 0);
        if (incomplete) {
          const msg = incompleteWarning(
            verified.missingCount,
            verified.recordedCount,
          );
          if (verified.chunks.length === 0) {
            setLoadError(msg || "Recording files are missing from disk.");
          } else {
            setAudioWarning(msg);
          }
        }
      } catch (e) {
        if (!cancelled) {
          setHasAudio(false);
          setLoadError(String(e));
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [meetingId]);

  const stopSources = useCallback(() => {
    for (const s of sourcesRef.current) {
      try {
        s.stop();
      } catch {
      /* already stopped */
      }
    }
    sourcesRef.current = [];
  }, []);

  const ensureCtx = useCallback(async () => {
    if (!audioCtxRef.current) {
      audioCtxRef.current = new AudioContext();
    }
    const ctx = audioCtxRef.current;
    if (ctx.state === "suspended") {
      await ctx.resume();
    }
    return ctx;
  }, []);

  const clearRaf = useCallback(() => {
    if (rafRef.current != null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
  }, []);

  const pause = useCallback(() => {
    playingRef.current = false;
    setPlaying(false);
    clearRaf();
    stopSources();
    fetchGenRef.current += 1;
    scheduleLockRef.current = false;
  }, [clearRaf, stopSources]);

  const stopWithWarning = useCallback(
    (message: string) => {
      pause();
      setAudioWarning(message);
    },
    [pause],
  );

  const scheduleWindow = useCallback(
    async (fromMs: number, gen: number): Promise<boolean> => {
      if (gen !== fetchGenRef.current || !playingRef.current) return false;
      if (scheduleLockRef.current) return false;
      if (fromMs + 1 < scheduledUntilRef.current) return false;

      const remaining = durationRef.current - fromMs;
      if (remaining <= 0) return false;

      const duration = Math.min(WINDOW_MS, remaining);

      if (
        hitsMissingSpan(
          fromMs,
          duration,
          sourceRef.current,
          missingSpansRef.current,
        )
      ) {
        stopWithWarning(
          "Playback stopped — an audio file is missing for this part of the meeting.",
        );
        return false;
      }

      scheduleLockRef.current = true;
      scheduledUntilRef.current = fromMs + duration;

      try {
        const ctx = await ensureCtx();
        if (gen !== fetchGenRef.current || !playingRef.current) return false;

        let decoded;
        try {
          decoded = await decodeMeetingAudioWindow(
            meetingId,
            sourceRef.current,
            Math.floor(fromMs),
            Math.floor(duration),
          );
        } catch (e) {
          console.error("decode_meeting_audio_window failed", e);
          stopWithWarning(
            "Playback stopped — could not decode meeting audio.",
          );
          return false;
        }
        if (gen !== fetchGenRef.current || !playingRef.current) return false;
        if (!decoded.samples.length) {
          stopWithWarning(
            "Playback stopped — no audio data for this part of the meeting.",
          );
          return false;
        }

        const buffer = pcmToAudioBuffer(
          ctx,
          decoded.samples,
          decoded.sampleRate || 16000,
        );
        const rate = speedRef.current;
        const node = ctx.createBufferSource();
        node.buffer = buffer;
        node.playbackRate.value = rate;
        node.connect(ctx.destination);

        const when = Math.max(ctx.currentTime + 0.01, nextStartAtRef.current);
        node.start(when);
        sourcesRef.current.push(node);
        nextStartAtRef.current = when + buffer.duration / rate;
        return true;
      } finally {
        scheduleLockRef.current = false;
      }
    },
    [ensureCtx, meetingId, stopWithWarning],
  );

  const pumpSchedule = useCallback(
    async (gen: number) => {
      while (
        gen === fetchGenRef.current &&
        playingRef.current &&
        scheduledUntilRef.current - tMsRef.current < PREFETCH_LEAD_MS
      ) {
        const from = scheduledUntilRef.current;
        if (from >= durationRef.current) break;
        const ok = await scheduleWindow(from, gen);
        if (!ok) break;
      }
    },
    [scheduleWindow],
  );

  const tick = useCallback(() => {
    if (!playingRef.current) return;

    const elapsedWall = performance.now() - playAnchorWallRef.current;
    const nextT = Math.min(
      playAnchorTRef.current + elapsedWall * speedRef.current,
      durationRef.current,
    );
    tMsRef.current = nextT;
    setTMs(nextT);

    const nearest = findNearestSegmentAtTime(
      segmentsRef.current,
      nextT,
      HIGHLIGHT_TOLERANCE_MS,
    );
    if (nearest && nearest.id !== lastFocusIdRef.current) {
      lastFocusIdRef.current = nearest.id;
      onFocusRef.current(nearest);
    }

    if (nextT >= durationRef.current - 16) {
      pause();
      return;
    }

    if (scheduledUntilRef.current - nextT < PREFETCH_LEAD_MS) {
      void pumpSchedule(fetchGenRef.current);
    }

    rafRef.current = requestAnimationFrame(tick);
  }, [pause, pumpSchedule]);

  const beginPlaybackFrom = useCallback(
    async (fromMs: number) => {
      const gen = ++fetchGenRef.current;
      stopSources();
      clearRaf();
      scheduleLockRef.current = false;

      const clamped = Math.max(0, Math.min(fromMs, durationRef.current));
      tMsRef.current = clamped;
      setTMs(clamped);
      scheduledUntilRef.current = clamped;

      const ctx = await ensureCtx();
      if (gen !== fetchGenRef.current) return;
      nextStartAtRef.current = ctx.currentTime;

      playingRef.current = true;
      setPlaying(true);
      playAnchorWallRef.current = performance.now();
      playAnchorTRef.current = clamped;

      await pumpSchedule(gen);
      if (gen !== fetchGenRef.current || !playingRef.current) return;
      clearRaf();
      rafRef.current = requestAnimationFrame(tick);
    },
    [clearRaf, ensureCtx, pumpSchedule, stopSources, tick],
  );

  const seekTo = useCallback(
    (nextMs: number) => {
      const next = Math.max(0, Math.min(nextMs, durationRef.current));
      tMsRef.current = next;
      setTMs(next);
      const nearest = findNearestSegmentAtTime(
        segmentsRef.current,
        next,
        HIGHLIGHT_TOLERANCE_MS,
      );
      if (nearest) {
        lastFocusIdRef.current = nearest.id;
        onFocusRef.current(nearest);
      }
      if (playingRef.current) {
        void beginPlaybackFrom(next);
      } else {
        stopSources();
        clearRaf();
        scheduledUntilRef.current = next;
      }
    },
    [beginPlaybackFrom, clearRaf, stopSources],
  );

  const togglePlay = useCallback(() => {
    if (playingRef.current) {
      pause();
      return;
    }
    void beginPlaybackFrom(tMsRef.current);
  }, [beginPlaybackFrom, pause]);

  const skipBy = useCallback(
    (deltaMs: number) => {
      seekTo(tMsRef.current + deltaMs);
    },
    [seekTo],
  );

  const cycleSpeed = useCallback(() => {
    const idx = SPEED_STEPS.indexOf(speedRef.current);
    const next = SPEED_STEPS[(idx + 1) % SPEED_STEPS.length]!;
    setSpeed(next);
    speedRef.current = next;
    if (playingRef.current) {
      void beginPlaybackFrom(tMsRef.current);
    }
  }, [beginPlaybackFrom]);

  useEffect(() => {
    if (playingRef.current) {
      void beginPlaybackFrom(tMsRef.current);
    }
  // Restart graph when synced source changes.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [audioSource]);

  useEffect(() => {
    if (seekToMs == null) return;
    onSeekConsumed();
    seekTo(seekToMs);
  }, [seekToMs, seekTo, onSeekConsumed]);

  useEffect(() => {
    return () => {
      pause();
      void audioCtxRef.current?.close();
      audioCtxRef.current = null;
    };
  }, [pause]);

  const onScrub = (value: number) => {
    seekTo(value);
  };

  // Source filter also drives the transcript timeline filter — meaningless
  // with an empty transcript, so it hides there (playback still works).
  const hasTranscript = segments.length > 0;

  if (hasAudio === null) {
    return (
      <div
        className={cn(floatingPlayerShellClass, "px-3 py-2")}
        aria-label="Meeting audio"
      >
        <div className="flex items-center justify-between gap-2">
          <p className="m-0 text-xs text-muted-foreground">Loading audio…</p>
          {hasTranscript ? (
            <AudioSourceFilterMenu
              filter={filter}
              onFilterChange={onFilterChange}
            />
          ) : null}
        </div>
      </div>
    );
  }

  if (!hasAudio) {
    return (
      <div className={floatingPlayerShellClass} aria-label="Meeting audio">
        <div className="flex items-start gap-2 px-3 py-2">
          <div className="min-w-0 flex-1">
            {loadError ? (
              <LiveNotice.Rail
                tone="warning"
                label={loadError}
                ariaLabel="Meeting audio warning"
              />
            ) : (
              <p className="m-0 text-xs text-muted-foreground">
                No recording for this meeting. Enable Save meeting audio in
                Settings for new meetings.
              </p>
            )}
          </div>
          {hasTranscript ? (
            <AudioSourceFilterMenu
              filter={filter}
              onFilterChange={onFilterChange}
            />
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <div
      className={floatingPlayerShellClass}
      aria-label="Meeting audio player"
    >
      {audioWarning ? (
        <LiveNotice.Rail
          tone="warning"
          label={audioWarning}
          ariaLabel="Meeting audio warning"
          onDismiss={() => setAudioWarning(null)}
        />
      ) : null}
      <div className="flex flex-wrap items-center gap-1.5 px-3 py-2">
      <AppTooltip label="Back 5 seconds">
        <Button
          type="button"
          variant="outline"
          size="icon-xs"
          aria-label="Back 5 seconds"
          onClick={() => skipBy(-SKIP_MS)}
        >
          <RotateCcw aria-hidden />
        </Button>
      </AppTooltip>
      <AppTooltip label={playing ? "Pause" : "Play"}>
        <Button
          type="button"
          variant="outline"
          size="icon-sm"
          aria-label={playing ? "Pause" : "Play"}
          onClick={togglePlay}
        >
          {playing ? <Pause aria-hidden /> : <Play aria-hidden />}
        </Button>
      </AppTooltip>
      <AppTooltip label="Forward 5 seconds">
        <Button
          type="button"
          variant="outline"
          size="icon-xs"
          aria-label="Forward 5 seconds"
          onClick={() => skipBy(SKIP_MS)}
        >
          <RotateCw aria-hidden />
        </Button>
      </AppTooltip>

      <span className="tabular-nums text-xs text-muted-foreground">
        {formatSegmentTimestamp(tMs)}
      </span>

      <input
        type="range"
        className="h-1.5 min-w-[5rem] flex-1 accent-primary"
        min={0}
        max={durationMs}
        step={50}
        value={Math.min(tMs, durationMs)}
        onChange={(e) => onScrub(Number(e.target.value))}
        aria-label="Seek meeting audio"
      />

      <span className="tabular-nums text-xs text-muted-foreground">
        {formatSegmentTimestamp(durationMs)}
      </span>

      <AppTooltip label={`Speed ${formatSpeedLabel(speed)} — click to change`}>
        <Button
          type="button"
          variant="outline"
          size="xs"
          className="min-w-[2.75rem] shrink-0 px-1.5 text-[10px] font-semibold tabular-nums leading-none"
          aria-label={`Playback speed ${formatSpeedLabel(speed)}. Click to change.`}
          onClick={cycleSpeed}
        >
          {formatSpeedLabel(speed)}
        </Button>
      </AppTooltip>

      {hasTranscript ? (
        <AudioSourceFilterMenu
          filter={filter}
          onFilterChange={onFilterChange}
        />
      ) : null}
      </div>
    </div>
  );
}
