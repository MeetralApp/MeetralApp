import { ChevronDown, ChevronRight, Folder } from "lucide-react";

import FolderInlineNameInput from "./FolderInlineNameInput";
import LibraryMeetingRow from "./LibraryMeetingRow";
import type { MeetingRecord } from "../lib/meetingTypes";
import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";

interface FolderBlockProps {
  folderId: string;
  name: string;
  expanded: boolean;
  selected: boolean;
  count: number;
  childMeetings: MeetingRecord[];
  activeMeetingId: string | null;
  depth: number;
  draft?: boolean;
  editing?: boolean;
  dragHandle?: React.ReactNode;
  onExpand: () => void;
  onSelect: () => void;
  onSelectMeeting: (meeting: MeetingRecord) => void;
  onNameSubmit?: (name: string) => void;
  onNameCancel?: () => void;
}

export default function LibraryFolderBlock({
  name,
  expanded,
  selected,
  count,
  childMeetings,
  activeMeetingId,
  depth,
  draft = false,
  editing = false,
  dragHandle = null,
  onExpand,
  onSelect,
  onSelectMeeting,
  onNameSubmit,
  onNameCancel,
}: FolderBlockProps) {
  const indent = 8 + depth * 16;
  const visibleMeetings = childMeetings.filter(
    (m) => !(m.status === "live" && m.id === activeMeetingId),
  );
  const isEditing = editing || draft;

  const mainContent = (
    <>
      <Folder
        size={14}
        aria-hidden
        strokeWidth={2}
        className="shrink-0 text-muted-foreground"
      />
      {isEditing && onNameSubmit && onNameCancel ? (
        <FolderInlineNameInput
          value={draft ? "" : name}
          onSubmit={onNameSubmit}
          onCancel={onNameCancel}
        />
      ) : (
        <span className="block min-w-0 truncate text-sm">
          {name}
        </span>
      )}
      <span className="min-w-5 shrink-0 text-right text-xs text-muted-foreground tabular-nums">
        {draft ? "" : count}
      </span>
    </>
  );

  const mainClassName = cn(
    "grid min-w-0 flex-1 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 overflow-hidden rounded-lg border border-transparent py-1.5 pr-2 pl-1",
    isEditing
      ? "border-ring/35 bg-accent/10"
      : "cursor-pointer bg-transparent text-left font-[inherit] text-foreground hover:bg-hover-surface hover:border-border",
    selected && !draft && !isEditing && "border-ring/25 bg-accent/10",
  );

  return (
    <div className="min-w-0 w-full">
      <div
        className={cn(
          "relative grid min-h-9 w-full min-w-0 items-center gap-0.5 rounded-lg",
          dragHandle
            ? "grid-cols-[auto_auto_minmax(0,1fr)]"
            : "grid-cols-[auto_minmax(0,1fr)]",
        )}
        style={{ paddingLeft: indent }}
      >
        {dragHandle}
        {draft ? (
          <span
            className="h-7 w-6 shrink-0"
            aria-hidden
          />
        ) : (
          <button
            type="button"
            className="grid h-7 w-6 shrink-0 enabled:cursor-pointer place-items-center rounded-md border-none bg-transparent text-muted-foreground hover:bg-hover-surface hover:text-foreground disabled:cursor-not-allowed"
            aria-expanded={expanded}
            aria-label={expanded ? `Collapse ${name}` : `Expand ${name}`}
            onClick={onExpand}
            disabled={isEditing}
          >
            {expanded ? (
              <ChevronDown size={14} aria-hidden strokeWidth={2} />
            ) : (
              <ChevronRight size={14} aria-hidden strokeWidth={2} />
            )}
          </button>
        )}
        {isEditing ? (
          <div className={mainClassName}>{mainContent}</div>
        ) : (
          <AppTooltip label={name} side="right">
            <button
              type="button"
              className={mainClassName}
              onClick={onSelect}
              aria-current={selected ? "true" : undefined}
            >
              {mainContent}
            </button>
          </AppTooltip>
        )}
      </div>
      {!draft &&
        expanded &&
        visibleMeetings.map((m) => (
          <LibraryMeetingRow
            key={m.id}
            meeting={m}
            depth={depth + 1}
            pinned={m.id === activeMeetingId && m.status === "live"}
            onSelect={() => onSelectMeeting(m)}
          />
        ))}
      {!draft && expanded && visibleMeetings.length === 0 && (
        <p
          className="m-0 px-2 py-1 text-sm text-muted-foreground"
          style={{ paddingLeft: indent + 24 }}
        >
          No meetings
        </p>
      )}
    </div>
  );
}
