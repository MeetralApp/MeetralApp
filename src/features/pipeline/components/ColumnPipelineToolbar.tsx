import { useState } from "react";
import { ChevronDown, Loader2 } from "lucide-react";
import type { AppStatus, AudioPathMode, ConfigView } from "@/shared/lib/types/pipeline";
import { isCustomVoiceOutput } from "@/shared/lib/types/pipeline";
import { getAudioPathMode, isColumnAwaitingDirectStandby, isDirectionDirect } from "../lib/pipelineStatus";
import { isColumnAudioFaultFromUi, type ColumnUiState } from "../lib/columnUi";
import { type PipelineModeOption } from "../lib/pipelineLabels";
import {
  inboundToolbarModeFromConfig,
  outboundToolbarModeFromConfig,
} from "../lib/pipelineLabels";
import {
  DirectPathIcon,
  TranslatePathIcon,
  pipelineOutputModeIcon,
} from "../lib/pipelineModeIcons";
import MicMuteButton from "@/features/audio/components/MicMuteButton";
import SpeakerMuteButton from "@/features/audio/components/SpeakerMuteButton";
import ColumnHeaderStatus from "./ColumnHeaderStatus";
import { Button } from "@/shared/ui/button";
import { ButtonGroup } from "@/shared/ui/button-group";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { useVoiceTtsStatus } from "@/features/voice/hooks/useVoiceTtsStatus";
import {
  pipelinePathTrackClass,
  pipelineToolbarClasses,
} from "@/features/pipeline/lib/pipelineColors";
import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";

export type { PipelineModeOption };

interface Props {
  title: string;
  direction: "outbound" | "inbound";
  status: AppStatus | null;
  config: ConfigView;
  columnUi: ColumnUiState;
  modeOptions: PipelineModeOption[];
  onAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onModeChange: (mode: string) => void | Promise<void>;
  micMuted?: boolean;
  micMuteDisabled?: boolean;
  onMicMuteToggle?: () => void;
  speakerMuted?: boolean;
  speakerMuteDisabled?: boolean;
  onSpeakerMuteToggle?: () => void;
  onLongReconnectRestored?: (columnTitle: string) => void;
  suppressSetupIdleChip?: boolean;
}

const directActiveClass = pipelineToolbarClasses.directActive;

const translateActiveClass = pipelineToolbarClasses.translateActive;

const translateActiveHoverClass = pipelineToolbarClasses.translateActiveHover;

const segmentIdleClass = pipelineToolbarClasses.segmentIdle;

/** Compact path segment — labels hide under @container max ~259px. */
const segmentBtn =
  "h-8 min-h-8 shrink-0 rounded-md gap-1.5 border-0 px-2.5 text-xs font-semibold shadow-none ring-0 focus-visible:ring-2 focus-visible:ring-ring/40 [&_svg]:size-3.5";

const segmentLabelClass = "hidden @[260px]:inline";

const translateSplitBtn =
  "h-8 min-h-8 min-w-0 gap-1.5 border-0 bg-transparent px-2.5 text-xs font-semibold shadow-none ring-0 transition-[opacity,color] duration-150 focus-visible:ring-2 focus-visible:ring-ring/40 [&_svg]:size-3.5";

const translateChevronBtn =
  "h-8 w-7 shrink-0 border-0 bg-transparent px-0 opacity-70 shadow-none ring-0 transition-[opacity,color] duration-150 hover:bg-transparent hover:opacity-100 focus-visible:ring-2 focus-visible:ring-ring/40";

export default function ColumnPipelineToolbar({
  title,
  direction,
  status,
  config,
  columnUi,
  modeOptions,
  onAudioPathChange,
  onModeChange,
  micMuted = false,
  micMuteDisabled,
  onMicMuteToggle,
  speakerMuted = false,
  speakerMuteDisabled,
  onSpeakerMuteToggle,
  onLongReconnectRestored,
  suppressSetupIdleChip = false,
}: Props) {
  const pathMode = getAudioPathMode(status, direction);
  // Prefer live runtime status so optimistic starting/stopping paints immediately.
  const state =
    (direction === "outbound" ? status?.outbound : status?.inbound) ??
    columnUi.pipeline;
  const canDirect = columnUi.canDirect;
  const translateSelected = pathMode === "translate";
  const cloneToolbarActive =
    (direction === "outbound"
      ? isCustomVoiceOutput(config.outboundVoiceOutput)
      : isCustomVoiceOutput(config.inboundVoiceOutput)) &&
    translateSelected &&
    (state === "active" || state === "starting" || state === "stopping");
  const { degradedMessage: voiceCloneDegraded } =
    useVoiceTtsStatus(cloneToolbarActive);
  const canTranslate = columnUi.canTranslate;
  const micDisabled = micMuteDisabled ?? !columnUi.muteEnabled;
  const speakerDisabled = speakerMuteDisabled ?? !columnUi.muteEnabled;
  const toolbarValue =
    direction === "outbound"
      ? outboundToolbarModeFromConfig(config)
      : inboundToolbarModeFromConfig(config);
  const ariaLabel = `${title} audio path`;
  const [outputMenuOpen, setOutputMenuOpen] = useState(false);

  const activeOption = modeOptions.find((opt) => opt.value === toolbarValue);
  const ModeIcon = pipelineOutputModeIcon(toolbarValue);
  const modeTip = activeOption?.title ?? activeOption?.label ?? "Output mode";
  const audioFault = isColumnAudioFaultFromUi(columnUi);
  const awaitingDirectStandby = isColumnAwaitingDirectStandby(
    config,
    status,
    direction,
    canDirect,
  );
  const transitioning = state === "starting" || state === "stopping";
  const starting = state === "starting";
  const stopping = state === "stopping";
  const translateDisabled =
    transitioning ||
    audioFault ||
    (!canTranslate && !translateSelected);
  const chevronDisabled = translateDisabled;
  // Default pathMode is "direct" when idle — don't paint Direct as selected
  // unless devices allow it or the pipeline is actually on Direct.
  const directSelected =
    pathMode === "direct" &&
    (canDirect || isDirectionDirect(status, direction));
  // Direct stays locked while standby restarts; Translate must remain clickable (P2).
  const directDisabled =
    !canDirect || awaitingDirectStandby || transitioning;

  const notesMode = config.sessionMode === "notes";
  const translateTitle = audioFault
    ? "Audio device disconnected — wait for reconnect or plug the device back in"
    : starting
      ? "Starting…"
      : stopping
        ? "Stopping…"
        : canTranslate
          ? notesMode
            ? "Start notes — STT captions + natural audio (no translation)"
            : "Translate"
          : "Add API key and audio devices in Settings";
  const directTitle = awaitingDirectStandby
    ? "Starting direct audio…"
    : transitioning
      ? stopping
        ? "Stopping…"
        : "Starting…"
      : canDirect
        ? notesMode
          ? "Direct — passthrough only, not capturing notes (no STT)"
          : "Direct — natural audio, no API usage"
        : "Configure audio devices in Settings";
  const translatePathLabel = notesMode ? "Notes" : "Translate";
  const translatePathAria = notesMode ? "Notes" : "Translate";
  const notesCapturing = notesMode && translateSelected;

  const handlePathChange = async (next: AudioPathMode) => {
    if (next === pathMode) return;
    if (next === "direct" && (!canDirect || awaitingDirectStandby)) return;
    if (next === "translate" && (!canTranslate || audioFault)) return;
    setOutputMenuOpen(false);
    await onAudioPathChange(next);
  };

  const handleMenuOpenChange = (open: boolean) => {
    if (open && chevronDisabled) return;
    setOutputMenuOpen(open);
  };

  const handleOutputSelect = async (value: string) => {
    if (modeOptions.find((o) => o.value === value)?.disabled) return;
    setOutputMenuOpen(false);
    // Persist mode/voice before starting Translate so the engine does not
    // snapshot stale Notes-clamped originalAudio / providerNative.
    await onModeChange(value);
    if (
      !translateSelected &&
      canTranslate &&
      !audioFault
    ) {
      await onAudioPathChange("translate");
    }
  };

  return (
    <div
      className="@container flex min-w-0 items-center gap-2 px-3 py-2"
      aria-label={ariaLabel}
    >
      <div
        className={pipelinePathTrackClass}
        role="group"
        aria-label={`${ariaLabel} mode`}
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
            <span className={segmentLabelClass}>Direct</span>
          </Button>
        </AppTooltip>

        <ButtonGroup
          className={cn(
            "group/translate min-w-0 overflow-hidden rounded-md",
            translateSelected
              ? cn(translateActiveClass, translateActiveHoverClass)
              : segmentIdleClass,
          )}
        >
          <AppTooltip label={translateTitle}>
            <Button
              type="button"
              variant="ghost"
              className={cn(
                translateSplitBtn,
                translateSelected
                  ? "text-inherit hover:bg-transparent hover:text-inherit hover:opacity-90"
                  : "hover:bg-transparent",
              )}
              aria-pressed={translateSelected}
              aria-label={translatePathAria}
              disabled={translateDisabled}
              onClick={() => void handlePathChange("translate")}
            >
              {starting || stopping ? (
                <Loader2 className="animate-spin" aria-hidden />
              ) : notesCapturing ? (
                <span
                  className="relative inline-flex size-3.5 items-center justify-center"
                  aria-hidden
                >
                  <TranslatePathIcon />
                  <span className="absolute -top-0.5 -right-0.5 size-1.5 animate-pulse rounded-full bg-destructive" />
                </span>
              ) : (
                <TranslatePathIcon aria-hidden />
              )}
              <span className={segmentLabelClass}>{translatePathLabel}</span>
            </Button>
          </AppTooltip>
          <AppTooltip label={modeTip}>
            <span
              className={cn(
                "hidden size-7 shrink-0 items-center justify-center @[260px]:inline-flex [&_svg]:size-3.5",
                translateSelected ? "text-inherit" : undefined,
              )}
            >
              <ModeIcon aria-hidden />
            </span>
          </AppTooltip>
          <DropdownMenu
            open={outputMenuOpen}
            onOpenChange={handleMenuOpenChange}
          >
            <DropdownMenuTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                className={cn(
                  translateChevronBtn,
                  translateSelected
                    ? "text-inherit hover:text-inherit"
                    : undefined,
                )}
                disabled={chevronDisabled}
                aria-label={`${title} output mode`}
              >
                <ChevronDown className="size-3.5" aria-hidden />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="min-w-[200px]">
              <DropdownMenuRadioGroup
                value={toolbarValue}
                onValueChange={handleOutputSelect}
              >
                {modeOptions.map((opt) => {
                  const Icon = pipelineOutputModeIcon(opt.value);
                  return (
                    <DropdownMenuRadioItem
                      key={opt.value}
                      value={opt.value}
                      disabled={opt.disabled}
                      className="gap-2"
                    >
                      <Icon
                        className="size-3.5 shrink-0 text-muted-foreground"
                        aria-hidden
                      />
                      <span className="min-w-0 flex-1">{opt.label}</span>
                    </DropdownMenuRadioItem>
                  );
                })}
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        </ButtonGroup>
      </div>

      <div className="ml-auto flex shrink-0 items-center gap-1">
        <ColumnHeaderStatus
          title={title}
          direction={direction}
          status={status}
          columnUi={columnUi}
          voiceCloneDegraded={voiceCloneDegraded}
          suppressSetupIdleChip={suppressSetupIdleChip}
          onLongReconnectRestored={onLongReconnectRestored}
        />
        {direction === "outbound" && onMicMuteToggle && (
          <MicMuteButton
            muted={micMuted}
            disabled={micDisabled}
            onToggle={onMicMuteToggle}
          />
        )}
        {direction === "inbound" && onSpeakerMuteToggle && (
          <SpeakerMuteButton
            muted={speakerMuted}
            disabled={speakerDisabled}
            onToggle={onSpeakerMuteToggle}
          />
        )}
      </div>
    </div>
  );
}
