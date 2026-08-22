import type { TranscriptEvent } from "@/shared/lib/types/pipeline";
import { peekNextSequence, type TranscriptDirection } from "./liveSegmentState";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

export interface UtteranceBlock {
  source?: string;
  translated?: string;
  connectionGap?: boolean;
  segmentId?: string;
}

export interface TranscriptView {
  history: UtteranceBlock[];
  live: UtteranceBlock | null;
}

export function segmentToUtteranceBlock(segment: TranscriptSegment): UtteranceBlock {
  return {
    segmentId: segment.id,
    source: segment.sourceText || undefined,
    translated: segment.translatedText || undefined,
    connectionGap: segment.connectionGap,
  };
}

function blocksMatch(a: UtteranceBlock, b: UtteranceBlock): boolean {
  const aSource = a.source?.trim() ?? "";
  const bSource = b.source?.trim() ?? "";
  if (aSource !== bSource) return false;

  const aTranslated = a.translated?.trim() ?? "";
  const bTranslated = b.translated?.trim() ?? "";
  if (aTranslated === bTranslated) return true;

  // Notes STT-only: BE mirrors translated=source on commit while liveSnapshot is
  // often source-only. Treat empty translated as source for equality.
  const aEffective = aTranslated || aSource;
  const bEffective = bTranslated || bSource;
  return aEffective === bEffective;
}

export function snapshotFromTranscriptEvent(event: TranscriptEvent): UtteranceBlock | null {
  const source = (event.liveSource ?? event.sourceText)?.trim();
  const translated = (event.liveTranslated ?? event.translatedText)?.trim();
  if (!source && !translated) return null;
  return {
    source: source || undefined,
    translated: translated || undefined,
  };
}

function lastTurnCompleteIndex(interim: TranscriptEvent[]): number {
  for (let i = interim.length - 1; i >= 0; i--) {
    if (interim[i]?.turnComplete) return i;
  }
  return -1;
}

/** Live tail from BE snapshot, with turnComplete handoff until segment-committed prune. */
export function resolveLiveTail(
  liveSnapshot: UtteranceBlock | null,
  committed: TranscriptSegment[],
  interim: TranscriptEvent[],
): UtteranceBlock | null {
  if (liveSnapshot && (liveSnapshot.source?.trim() || liveSnapshot.translated?.trim())) {
    return liveSnapshot;
  }
  if (committed.length === 0 || lastTurnCompleteIndex(interim) < 0) {
    return null;
  }
  return segmentToUtteranceBlock(committed[committed.length - 1]!);
}

/** Keep interim events after the last turnComplete (preserve in-flight next utterance). */
export function pruneInterimAfterSegmentCommit(
  interim: TranscriptEvent[],
): TranscriptEvent[] {
  let lastTurnComplete = -1;
  for (let i = interim.length - 1; i >= 0; i--) {
    if (interim[i]?.turnComplete) {
      lastTurnComplete = i;
      break;
    }
  }
  return lastTurnComplete >= 0 ? interim.slice(lastTurnComplete + 1) : interim;
}

export type LiveDisplayRow =
  | { kind: "committed"; segment: TranscriptSegment }
  | { kind: "live"; block: UtteranceBlock; sequence: number };

export interface LiveDisplayState {
  rows: LiveDisplayRow[];
  live: UtteranceBlock | null;
}

function liveRowSequence(
  committed: TranscriptSegment[],
  live: UtteranceBlock,
): number {
  if (committed.length > 0) {
    const last = committed[committed.length - 1]!;
    if (blocksMatch(live, segmentToUtteranceBlock(last))) {
      return last.sequence;
    }
  }
  return peekNextSequence(committed);
}

export function buildLiveDisplayState(
  committed: TranscriptSegment[],
  liveSnapshot: UtteranceBlock | null,
  interim: TranscriptEvent[],
): LiveDisplayState {
  const live = resolveLiveTail(liveSnapshot, committed, interim);
  let visibleCommitted = committed;

  if (live && committed.length > 0) {
    const last = committed[committed.length - 1]!;
    if (blocksMatch(live, segmentToUtteranceBlock(last))) {
      visibleCommitted = committed.slice(0, -1);
    }
  }

  const rows: LiveDisplayRow[] = visibleCommitted.map((segment) => ({
    kind: "committed",
    segment,
  }));

  if (live) {
    rows.push({
      kind: "live",
      block: live,
      sequence: liveRowSequence(committed, live),
    });
  }

  return { rows, live };
}

export function liveTailFingerprint(rows: LiveDisplayRow[]): string | null {
  const tail = rows[rows.length - 1];
  if (!tail || tail.kind !== "live") return null;
  return `${tail.block.source ?? ""}\0${tail.block.translated ?? ""}`;
}

export function buildLiveTranscriptView(
  committed: TranscriptSegment[],
  liveSnapshot: UtteranceBlock | null,
  interim: TranscriptEvent[],
): TranscriptView {
  const { live } = buildLiveDisplayState(committed, liveSnapshot, interim);
  let history = committed.map(segmentToUtteranceBlock);

  if (live && committed.length > 0) {
    const last = committed[committed.length - 1]!;
    if (blocksMatch(live, segmentToUtteranceBlock(last))) {
      history = history.slice(0, -1);
    }
  }

  return { history, live };
}

export function buildLiveTranscriptRows(
  committed: TranscriptSegment[],
  liveSnapshot: UtteranceBlock | null,
  interim: TranscriptEvent[],
): LiveDisplayRow[] {
  return buildLiveDisplayState(committed, liveSnapshot, interim).rows;
}

export function liveDisplayRowKey(
  row: LiveDisplayRow,
  direction: TranscriptDirection,
): string {
  const sequence = row.kind === "committed" ? row.segment.sequence : row.sequence;
  return `seq-${direction}-${sequence}`;
}

export function visibleCommittedSegments(
  committed: TranscriptSegment[],
  liveSnapshot: UtteranceBlock | null,
  interim: TranscriptEvent[],
): TranscriptSegment[] {
  return buildLiveTranscriptRows(committed, liveSnapshot, interim)
    .filter((row): row is Extract<LiveDisplayRow, { kind: "committed" }> => row.kind === "committed")
    .map((row) => row.segment);
}

export function utteranceKey(block: UtteranceBlock, index: number): string {
  if (block.segmentId) return block.segmentId;
  const source = block.source?.trim() ?? "";
  const translated = block.translated?.trim() ?? "";
  if (source || translated) {
    return `${index}:${source.slice(0, 32)}:${translated.slice(0, 32)}:${block.connectionGap ? "gap" : ""}`;
  }
  return `utterance-${index}`;
}
