import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

export type TranscriptDirection = "outbound" | "inbound";

export function isClientSegmentId(id: string): boolean {
  return id.startsWith("client-");
}

export function peekNextSequence(committed: TranscriptSegment[]): number {
  if (committed.length === 0) return 1;
  return Math.max(...committed.map((s) => s.sequence)) + 1;
}

export function sortSegmentsAsc(segments: TranscriptSegment[]): TranscriptSegment[] {
  return [...segments].sort((a, b) => a.sequence - b.sequence);
}

export function dedupeSegmentsById(segments: TranscriptSegment[]): TranscriptSegment[] {
  const byId = new Map<string, TranscriptSegment>();
  for (const segment of segments) {
    byId.set(segment.id, segment);
  }
  return sortSegmentsAsc([...byId.values()]);
}

/**
* One row per (direction, sequence). Prefer authoritative (non-`client-*`) over
* optimistic — liveDisplayRowKey is sequence-only, so same-seq duplicates blank
* older virtualized rows when the list reshuffles on the next append.
*/
export function dedupeSegmentsBySequence(
  segments: TranscriptSegment[],
): TranscriptSegment[] {
  const byKey = new Map<string, TranscriptSegment>();
  for (const segment of segments) {
    const key = `${segment.direction}:${segment.sequence}`;
    const existing = byKey.get(key);
    if (!existing) {
      byKey.set(key, segment);
      continue;
    }
    const existingClient = isClientSegmentId(existing.id);
    const incomingClient = isClientSegmentId(segment.id);
    if (existingClient && !incomingClient) {
      byKey.set(key, segment);
    } else if (existingClient === incomingClient) {
      byKey.set(key, segment);
    }
  // else: keep authoritative existing when incoming is optimistic
  }
  return sortSegmentsAsc([...byKey.values()]);
}

export function mergeHydratedSegments(
  existing: TranscriptSegment[],
  incoming: TranscriptSegment[],
): TranscriptSegment[] {
  return dedupeSegmentsBySequence([...existing, ...incoming]);
}

export function prependOlderSegments(
  existing: TranscriptSegment[],
  older: TranscriptSegment[],
): TranscriptSegment[] {
  return dedupeSegmentsBySequence([...older, ...existing]);
}

export function insertSegmentSorted(
  committed: TranscriptSegment[],
  segment: TranscriptSegment,
): TranscriptSegment[] {
  const existingIndex = committed.findIndex((s) => s.id === segment.id);
  if (existingIndex >= 0) {
    const next = [...committed];
    next[existingIndex] = segment;
    return next;
  }
  let insertAt = committed.length;
  for (let i = 0; i < committed.length; i++) {
    if (committed[i]!.sequence > segment.sequence) {
      insertAt = i;
      break;
    }
  }
  const next = [...committed];
  next.splice(insertAt, 0, segment);
  return next;
}

export function replaceOptimisticOrAppend(
  committed: TranscriptSegment[],
  segment: TranscriptSegment,
): TranscriptSegment[] {
  // Sequence-unique merge: drop optimistic/stray dups for this sequence and any
  // leftover same-seq pairs (e.g. hydrate raced with preview). Prefer the
  // authoritative segment being committed.
  return dedupeSegmentsBySequence([...committed, segment]);
}

export function createOptimisticSegment(
  direction: TranscriptDirection,
  clientSeq: number,
  meetingId: string,
  sequence: number,
  sourceText: string,
  translatedText: string,
  connectionGap: boolean,
): TranscriptSegment {
  return {
    id: `client-${direction}-${clientSeq}`,
    meetingId,
    direction,
    sequence,
    sourceText,
    translatedText,
    startedAtMs: 0,
    endedAtMs: 0,
    connectionGap,
  };
}
