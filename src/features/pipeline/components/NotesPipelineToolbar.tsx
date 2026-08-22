import { Loader2 } from "lucide-react";
import type { AppStatus, AudioPathMode, ConfigView } from "@/shared/lib/types/pipeline";
import {
  getAudioPathMode,
  isColumnAwaitingDirectStandby,
  isDirectionDirect,
} from "../lib/pipelineStatus";
import { isColumnAudioFaultFromUi, type ColumnUiState } from "../lib/columnUi";
import { DirectPathIcon, NotesPathIcon } from "../lib/pipelineModeIcons";
import MicMuteButton from "@/features/audio/components/MicMuteButton";
import SpeakerMuteButton from "@/features/audio/components/SpeakerMuteButton";
import ColumnHeaderStatus from "./ColumnHeaderStatus";
import { Button } from "@/shared/ui/button";
import {
  pipelinePathTrackClass,
  pipelineToolbarClasses,
} from "@/features/pipeline/lib/pipelineColors";
import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";

interface Props {
  status: AppStatus | null;
  config: ConfigView;
  outboundColumn: ColumnUiState;
  inboundColumn: ColumnUiState;
  onOutboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onInboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  micMuted?: boolean;
  onMicMuteToggle?: () => void;
  speakerMuted?: boolean;
  onSpeakerMuteToggle?: () => void;
  onLongReconnectRestored?: (columnTitle: string) => void;
  suppressSetupIdleChip?: boolean;
}

const directActiveClass = pipelineToolbarClasses.directActive;
const notesActiveClass = pipelineToolbarClasses.notesActive;
const notesActiveHoverClass = pipelineToolbarClasses.notesActiveHover;
const segmentIdleClass = pipelineToolbarClasses.segmentIdle;

const segmentBtn =
  "h-8 min-h-8 shrink-0 rounded-md gap-1.5 border-0 px-2.5 text-xs font-semibold shadow-none ring-0 focus-visible:ring-2 focus-visible:ring-ring/40 [&_svg]:size-3.5";

/**
* Notes session: one Direct | Notes control that toggles both audio paths.
* 3-zone rail — mic (You) | shared path | speaker+status (Meeting).
*/
export default function NotesPipelineToolbar({
  status,
  config,
  outboundColumn,
  inboundColumn,
  onOutboundAudioPathChange,
  onInboundAudioPathChange,
  micMuted = false,
  onMicMuteToggle,
  speakerMuted = false,
  onSpeakerMuteToggle,
  onLongReconnectRestored,
  suppressSetupIdleChip = false,
}: Props) {
  const outboundPath = getAudioPathMode(status, "outbound");
  const inboundPath = getAudioPathMode(status, "inbound");
  const notesSelected =
    outboundPath === "translate" || inboundPath === "translate";
  const directSelected =
    !notesSelected &&
    ((outboundPath === "direct" &&
      (outboundColumn.canDirect || isDirectionDirect(status, "outbound"))) ||
      (inboundPath === "direct" &&
        (inboundColumn.canDirect || isDirectionDirect(status, "inbound"))));

  const canDirect = outboundColumn.canDirect && inboundColumn.canDirect;
  const canTranslate = outboundColumn.canTranslate && inboundColumn.canTranslate;
  const audioFault =
    isColumnAudioFaultFromUi(outboundColumn) ||
    isColumnAudioFaultFromUi(inboundColumn);
  const awaitingDirectStandby =
    isColumnAwaitingDirectStandby(
      config,
      status,
      "outbound",
      outboundColumn.canDirect,
    ) ||
    isColumnAwaitingDirectStandby(
      config,
      status,
      "inbound",
      inboundColumn.canDirect,
    );
  const starting =
    status?.outbound === "starting" || status?.inbound === "starting";
  const stopping =
    status?.outbound === "stopping" || status?.inbound === "stopping";
  const transitioning = starting || stopping;
  const directDisabled = !canDirect || awaitingDirectStandby || transitioning;
  const notesDisabled =
    transitioning || audioFault || (!canTranslate && !notesSelected);

  const notesTitle = audioFault
    ? "Audio device disconnected — wait for reconnect or plug the device back in"
    : starting
      ? "Starting…"
      : stopping
        ? "Stopping…"
        : canTranslate
          ? "Start notes — STT captions + natural audio (no translation)"
          : "Add API key and audio devices in Settings";
  const directTitle = awaitingDirectStandby
    ? "Starting direct audio…"
    : transitioning
      ? stopping
        ? "Stopping…"
        : "Starting…"
      : canDirect
        ? "Direct — passthrough only, not capturing notes (no STT)"
        : "Configure audio devices in Settings";

  const statusColumn =
    isColumnAudioFaultFromUi(outboundColumn) ||
    status?.outbound === "starting" ||
    status?.outbound === "stopping" ||
    status?.outbound === "active"
      ? { direction: "outbound" as const, columnUi: outboundColumn, title: "You" }
      : isColumnAudioFaultFromUi(inboundColumn) ||
          status?.inbound === "starting" ||
          status?.inbound === "stopping" ||
          status?.inbound === "active"
        ? {
            direction: "inbound" as const,
            columnUi: inboundColumn,
            title: "Meeting",
          }
        : {
            direction: "outbound" as const,
            columnUi: outboundColumn,
            title: "Notes",
          };

  const handlePathChange = async (next: AudioPathMode) => {
    if (next === "direct" && directDisabled) return;
    if (next === "translate" && notesDisabled) return;
    if (next === "translate" && notesSelected) return;
    if (next === "direct" && directSelected) return;
    // Run both sides even if one rejects — each path handler surfaces its own error.
    await Promise.allSettled([
      onOutboundAudioPathChange(next),
      onInboundAudioPathChange(next),
    ]);
  };

  return (
    <div
      className="flex min-w-0 items-center gap-2 px-3 py-2"
      aria-label="Notes audio path"
    >
      <div className="flex min-w-0 flex-1 items-center justify-start">
        {onMicMuteToggle ? (
          <MicMuteButton
            muted={micMuted}
            disabled={!outboundColumn.muteEnabled}
            onToggle={onMicMuteToggle}
          />
        ) : null}
      </div>

      <div
        className={cn(pipelinePathTrackClass, "shrink-0")}
        role="group"
        aria-label="Notes audio path mode"
      >
        <AppTooltip label={directTitle}>
          <Button
            type="button"
            variant="ghost"
            className={cn(
              segmentBtn,
              directSelected ? directActiveClass : segmentIdleClass,
            )}
            aria-pressed={directSelected}
            aria-label="Direct"
            disabled={directDisabled}
            onClick={() => void handlePathChange("direct")}
          >
            <DirectPathIcon aria-hidden />
            <span>Direct</span>
          </Button>
        </AppTooltip>

        <AppTooltip label={notesTitle}>
          <Button
            type="button"
            variant="ghost"
            className={cn(
              segmentBtn,
              notesSelected
                ? cn(notesActiveClass, notesActiveHoverClass)
                : segmentIdleClass,
            )}
            aria-pressed={notesSelected}
            aria-label="Notes"
            disabled={notesDisabled}
            onClick={() => void handlePathChange("translate")}
          >
            {starting || stopping ? (
              <Loader2 className="animate-spin" aria-hidden />
            ) : (
              <NotesPathIcon aria-hidden />
            )}
            <span>Notes</span>
          </Button>
        </AppTooltip>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-end gap-1">
        <ColumnHeaderStatus
          title={statusColumn.title}
          direction={statusColumn.direction}
          status={status}
          columnUi={statusColumn.columnUi}
          voiceCloneDegraded={null}
          suppressSetupIdleChip={suppressSetupIdleChip}
          onLongReconnectRestored={onLongReconnectRestored}
        />
        {onSpeakerMuteToggle ? (
          <SpeakerMuteButton
            muted={speakerMuted}
            disabled={!inboundColumn.muteEnabled}
            onToggle={onSpeakerMuteToggle}
          />
        ) : null}
      </div>
    </div>
  );
}
