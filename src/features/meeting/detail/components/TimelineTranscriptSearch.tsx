import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { ChevronDown, ChevronUp, Search, X } from "lucide-react";

import { segmentMatchesQuery } from "@/features/meeting/library/lib/highlightSnippet";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";
import { pipelineToolbarClasses } from "@/features/pipeline/lib/pipelineColors";
import AppTooltip from "@/shared/components/AppTooltip";

const DEBOUNCE_MS = 200;

const floatingSearchShellClass = cn(
  "flex h-9 items-stretch overflow-hidden rounded-lg border border-border/80",
  "bg-secondary shadow-[var(--pane-elevated)] ring-1 ring-foreground/10",
);

interface Props {
  meetingId: string;
  /** Chronological segments for this meeting (find order). */
  segments: TranscriptSegment[];
  onJumpToSegment: (segment: TranscriptSegment) => void;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Find-in-page: floating bar inside transcript panel when open. */
export default function TimelineTranscriptSearch({
  meetingId,
  segments,
  onJumpToSegment,
  open,
  onOpenChange,
}: Props) {
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [matchIndex, setMatchIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const lastJumpedIdRef = useRef<string | null>(null);
  const onJumpRef = useRef(onJumpToSegment);
  onJumpRef.current = onJumpToSegment;

  useEffect(() => {
    setQuery("");
    setDebounced("");
    setMatchIndex(0);
    lastJumpedIdRef.current = null;
  }, [meetingId]);

  useEffect(() => {
    if (!open) {
      setQuery("");
      setDebounced("");
      setMatchIndex(0);
      lastJumpedIdRef.current = null;
      return;
    }
    const id = window.requestAnimationFrame(() => {
      inputRef.current?.focus();
    });
    return () => window.cancelAnimationFrame(id);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const timer = window.setTimeout(() => {
      setDebounced(query.trim());
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query, open]);

  const matches = useMemo(() => {
    if (!open || !debounced) return [];
    return segments.filter((s) => segmentMatchesQuery(s, debounced));
  }, [segments, debounced, open]);

  const active = open && debounced.length > 0;
  const matchCount = matches.length;
  const safeIndex =
    matchCount === 0 ? 0 : Math.min(matchIndex, matchCount - 1);

  useEffect(() => {
    if (!active || matchCount === 0) {
      lastJumpedIdRef.current = null;
      return;
    }
    setMatchIndex((i) => (i >= matchCount ? 0 : i));
  }, [active, matchCount]);

  useEffect(() => {
    if (!active || matchCount === 0) return;
    const target = matches[safeIndex];
    if (!target) return;
    if (lastJumpedIdRef.current === target.id) return;
    lastJumpedIdRef.current = target.id;
    onJumpRef.current(target);
  }, [active, matchCount, matches, safeIndex]);

  const goTo = (nextIndex: number) => {
    if (matchCount === 0) return;
    const wrapped = ((nextIndex % matchCount) + matchCount) % matchCount;
    lastJumpedIdRef.current = null;
    setMatchIndex(wrapped);
  };

  const close = () => {
    setQuery("");
    setDebounced("");
    setMatchIndex(0);
    lastJumpedIdRef.current = null;
    onOpenChange(false);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (matchCount === 0) return;
      goTo(event.shiftKey ? safeIndex - 1 : safeIndex + 1);
    }
  };

  if (!open) return null;

  const showMatchCount = query.trim().length > 0;
  const matchLabel =
    matchCount === 0 ? "0/0" : `${safeIndex + 1}/${matchCount}`;

  return (
    <div className="pointer-events-none absolute inset-x-0 top-0 z-[8] flex justify-end pt-2 pr-4 pb-2 pl-2">
      <div
        className={cn(
          floatingSearchShellClass,
          "pointer-events-auto w-[min(20rem,calc(100%-0.25rem))]",
        )}
        role="search"
        aria-label="Find in transcript"
      >
        <Input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder=""
          className={cn(
            "h-auto min-w-0 flex-1 rounded-none border-0 bg-transparent",
            "px-3 py-0 text-sm shadow-none focus-visible:ring-0",
          )}
          aria-label="Find in this meeting transcript"
          aria-describedby={showMatchCount ? "timeline-find-status" : undefined}
        />
        {showMatchCount ? (
          <span
            id="timeline-find-status"
            className="shrink-0 self-center px-1.5 text-xs tabular-nums text-muted-foreground"
            aria-live="polite"
          >
            {matchLabel}
          </span>
        ) : null}
        <span className="w-px shrink-0 self-stretch bg-border/80" aria-hidden />
        <div className="flex shrink-0 items-center gap-0.5 px-1">
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            className="text-muted-foreground hover:text-foreground"
            aria-label="Previous match"
            disabled={!active || matchCount === 0}
            onClick={() => goTo(safeIndex - 1)}
          >
            <ChevronUp aria-hidden />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            className="text-muted-foreground hover:text-foreground"
            aria-label="Next match"
            disabled={!active || matchCount === 0}
            onClick={() => goTo(safeIndex + 1)}
          >
            <ChevronDown aria-hidden />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            className="text-muted-foreground hover:text-foreground"
            aria-label="Close search"
            onClick={close}
          >
            <X aria-hidden />
          </Button>
        </div>
      </div>
    </div>
  );
}

/** Compact header trigger — toggles the floating find bar. */
export function TimelineSearchTrigger({
  open,
  onToggle,
}: {
  open: boolean;
  onToggle: () => void;
}) {
  const label = open ? "Close find in transcript" : "Find in transcript";
  return (
    <AppTooltip label={label}>
      <Button
        type="button"
        variant="outline"
        size="icon-sm"
        className={cn(
          pipelineToolbarClasses.paneToolbarActionChip,
          open && "ring-1 ring-border/80 dark:ring-white/25",
        )}
        aria-label={label}
        aria-expanded={open}
        aria-pressed={open}
        onClick={onToggle}
      >
        <Search aria-hidden strokeWidth={2} />
      </Button>
    </AppTooltip>
  );
}
