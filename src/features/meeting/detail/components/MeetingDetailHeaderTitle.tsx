import { useEffect, useState } from "react";
import { ChevronDown } from "lucide-react";

import MeetingDetailActions from "./MeetingDetailActions";
import { Badge } from "@/shared/ui/badge";
import AppTooltip from "@/shared/components/AppTooltip";
import { useMeetings } from "@/features/meeting/library/hooks/useMeetings";
import { getMeeting } from "@/features/meeting/library/lib/meetingApi";
import {
  formatMeetingDetailChipMeta,
  formatMeetingDetailChipTooltip,
  meetingDetailDisplayTitle,
} from "@/features/meeting/library/lib/meetingDisplay";
import type { MeetingRecord } from "@/features/meeting/library/lib/meetingTypes";
import { headerChipTriggerClass } from "@/shared/lib/headerChrome";
import { cn } from "@/shared/lib/utils";

interface Props {
  meetingId: string;
  onDeleted?: () => void;
}

function MeetingChipTooltipLabel({
  name,
  when,
  languages,
}: {
  name: string;
  when: string;
  languages: string;
}) {
  return (
    <div className="flex min-w-0 flex-col gap-0.5 text-left">
      <p className="m-0 font-medium text-popover-foreground">{name}</p>
      <p className="m-0 text-muted-foreground">{when}</p>
      <p className="m-0 text-muted-foreground">{languages}</p>
    </div>
  );
}

/** Compact meeting chip for the app header center (between Live back and Settings). */
export default function MeetingDetailHeaderTitle({
  meetingId,
  onDeleted,
}: Props) {
  const {
    meetings,
    folders,
    renameMeetingTitle,
    moveMeetingToFolder,
    removeMeeting,
    refreshFolders,
  } = useMeetings();

  const fromList = meetings.find((m) => m.id === meetingId) ?? null;
  const [fetched, setFetched] = useState<MeetingRecord | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (fromList) {
      setFetched(null);
      return;
    }
    let cancelled = false;
    void getMeeting(meetingId)
      .then((m) => {
        if (!cancelled) setFetched(m);
      })
      .catch(() => {
        if (!cancelled) setFetched(null);
      });
    return () => {
      cancelled = true;
    };
  }, [meetingId, fromList]);

  const meeting = fromList ?? fetched;
  if (!meeting) return null;

  const displayTitle = meetingDetailDisplayTitle(meeting);
  const metaLine = formatMeetingDetailChipMeta(meeting);
  const tooltipLines = formatMeetingDetailChipTooltip(meeting);
  const tooltipLabel = (
    <MeetingChipTooltipLabel
      name={tooltipLines.name}
      when={tooltipLines.when}
      languages={tooltipLines.languages}
    />
  );
  const tooltipAria = `${tooltipLines.name}. ${tooltipLines.when}. ${tooltipLines.languages}`;
  const isLive = meeting.status === "live";
  const chipClass = cn(
    headerChipTriggerClass(
      "flex h-auto min-h-8 w-full min-w-0 items-stretch overflow-hidden p-0 font-normal text-foreground",
    ),
    menuOpen && "border-border/80 bg-secondary",
    isLive && "cursor-default",
  );

  const chipFace = (
    <span className="flex min-w-0 flex-1 items-center gap-1 py-1 pl-2.5 pr-2">
      <span className="flex min-w-0 flex-1 flex-col gap-0 text-left">
        <span className="truncate text-xs font-semibold leading-tight text-foreground">
          {displayTitle}
        </span>
        <span className="truncate text-[0.65rem] leading-tight text-muted-foreground">
          {metaLine}
        </span>
      </span>
      {!isLive ? (
        <ChevronDown
          className="size-3.5 shrink-0 text-muted-foreground"
          aria-hidden
          strokeWidth={2}
        />
      ) : null}
    </span>
  );

  return (
    <div className="mx-auto flex w-full max-w-[min(24rem,50vw)] min-w-0 items-center gap-1.5">
      <div className="relative min-w-0 flex-1">
        {isLive ? (
          <AppTooltip label={tooltipLabel} contentClassName="max-w-xs">
            <div className={chipClass} aria-label={tooltipAria}>
              {chipFace}
            </div>
          </AppTooltip>
        ) : (
          <MeetingDetailActions
            meeting={meeting}
            folders={folders}
            contentAlign="center"
            tooltipLabel={tooltipLabel}
            onOpenChange={setMenuOpen}
            onRename={async (title) => {
              await renameMeetingTitle(meeting.id, title);
            }}
            onMove={async (folderId) => {
              await moveMeetingToFolder(meeting.id, folderId);
            }}
            onDelete={async () => {
              await removeMeeting(meeting.id);
              onDeleted?.();
            }}
            onRefreshFolders={refreshFolders}
            trigger={
              <button
                type="button"
                className={chipClass}
                aria-label={`${tooltipAria}. Open meeting menu`}
                aria-haspopup="menu"
                aria-expanded={menuOpen}
                onMouseDown={(e) => e.stopPropagation()}
              >
                {chipFace}
              </button>
            }
          />
        )}
      </div>

      {isLive ? (
        <Badge
          variant="secondary"
          className="shrink-0 border-transparent bg-emerald-500/20 text-emerald-300"
        >
          Live
        </Badge>
      ) : null}
    </div>
  );
}
