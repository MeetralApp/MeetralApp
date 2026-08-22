import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

export type TimelineDirectionFilter = "all" | "outbound" | "inbound";

export const TIMELINE_FILTER_OPTIONS: {
  id: TimelineDirectionFilter;
  label: string;
  title: string;
}[] = [
  {
    id: "all",
    label: "All",
    title: "Show all · play mixed room",
  },
  {
    id: "outbound",
    label: "You",
    title: "Show and play your mic only",
  },
  {
    id: "inbound",
    label: "Meeting",
    title: "Show and play meeting capture only",
  },
];

/** Sort by meeting-relative start time, then sequence for stable ties. */
export function sortSegmentsChronologically(
  segments: TranscriptSegment[],
): TranscriptSegment[] {
  return [...segments].sort((a, b) => {
    if (a.startedAtMs !== b.startedAtMs) {
      return a.startedAtMs - b.startedAtMs;
    }
    return a.sequence - b.sequence;
  });
}

export function filterSegmentsByDirection(
  segments: TranscriptSegment[],
  filter: TimelineDirectionFilter,
): TranscriptSegment[] {
  if (filter === "all") return segments;
  return segments.filter((s) => s.direction === filter);
}

/** Format meeting-relative ms as m:ss or h:mm:ss. */
export function formatSegmentTimestamp(startedAtMs: number): string {
  const totalSec = Math.max(0, Math.floor(startedAtMs / 1000));
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const seconds = totalSec % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function timelineSpeakerLabel(
  direction: "outbound" | "inbound",
): string {
  return direction === "outbound" ? "You" : "Meeting";
}

export function timelineFilterLabel(filter: TimelineDirectionFilter): string {
  return (
    TIMELINE_FILTER_OPTIONS.find((o) => o.id === filter)?.label ?? "All"
  );
}

/** Map timeline filter → playback source (All plays Room mix). */
export function audioSourceForFilter(
  filter: TimelineDirectionFilter,
): "room" | "you" | "meeting" {
  if (filter === "outbound") return "you";
  if (filter === "inbound") return "meeting";
  return "room";
}

/** Nearest segment to meeting-relative `tMs` within `toleranceMs` (default ±500). */
export function findNearestSegmentAtTime(
  segments: TranscriptSegment[],
  tMs: number,
  toleranceMs = 500,
): TranscriptSegment | null {
  if (segments.length === 0) return null;
  let best: TranscriptSegment | null = null;
  let bestDist = Number.POSITIVE_INFINITY;
  for (const seg of segments) {
    const dist = Math.abs(seg.startedAtMs - tMs);
    if (dist < bestDist) {
      bestDist = dist;
      best = seg;
    }
  }
  if (!best || bestDist > toleranceMs) return null;
  return best;
}
