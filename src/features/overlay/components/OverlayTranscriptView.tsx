import { ArrowDown, GripVertical, SunMedium, X } from "lucide-react";
import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  AppStatus,
  AudioPathMode,
  OverlaySettings,
  TranscriptLayout,
} from "@/shared/lib/types/pipeline";
import type { PipelineModeOption } from "@/features/pipeline/lib/pipelineLabels";
import { cn } from "@/shared/lib/utils";
import { useStickToBottomScroll } from "@/shared/hooks/useStickToBottomScroll";
import { scrollElementToBottom } from "@/features/pipeline/lib/stickToBottomScroll";
import { Button } from "@/shared/ui/button";
import AppTooltip from "@/shared/components/AppTooltip";
import type { OverlayTailRow } from "../lib/mockTranscript";
import { filterOverlayRows } from "../lib/overlayTail";
import { overlayHoldKey } from "../lib/platformLabel";
import type { OverlayColumnRuntime } from "../lib/overlayRuntimeTypes";
import OverlayColumnControls from "./OverlayColumnControls";
import OverlayResizeHandle from "./OverlayResizeHandle";
import { notesSegmentText } from "@/features/pipeline/lib/sessionMode";
import {
  DirectPathIcon,
  NotesPathIcon,
} from "@/features/pipeline/lib/pipelineModeIcons";
import MicMuteButton from "@/features/audio/components/MicMuteButton";
import SpeakerMuteButton from "@/features/audio/components/SpeakerMuteButton";

export type OverlayControlsProps = {
  status: AppStatus | null;
  outbound: OverlayColumnRuntime;
  inbound: OverlayColumnRuntime;
  micMuted: boolean;
  speakerMuted: boolean;
  outboundToolbarMode: string;
  inboundMode: string;
  outboundModeOptions: PipelineModeOption[];
  inboundModeOptions: PipelineModeOption[];
  customActive: boolean;
  sessionMode?: "interpreter" | "notes";
  onMicToggle: () => void;
  onSpeakerToggle: () => void;
  onOutboundPath: (mode: AudioPathMode) => void;
  onInboundPath: (mode: AudioPathMode) => void;
  onOutputModeChange: (
    direction: "outbound" | "inbound",
    value: string,
  ) => void;
  onCycleOpacity: () => void;
};

type Props = {
  rows: OverlayTailRow[];
  overlay: OverlaySettings;
  transcriptLayout: TranscriptLayout;
  notesMode?: boolean;
  /** Header title — active meeting name when available. */
  title?: string;
  onHide?: () => void;
  compact?: boolean;
  controls?: OverlayControlsProps | null;
};

function useOverlayColumnScroll(rows: OverlayTailRow[], notesMode: boolean) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const historyLength = rows.filter((r) => !r.live).length;
  const liveRow = rows.find((r) => r.live);
  const hasLiveRow = liveRow != null;
  const liveTailText = liveRow
    ? notesMode
      ? notesSegmentText(liveRow.sourceText, liveRow.translatedText)
      : `${liveRow.sourceText}\0${liveRow.translatedText}`
    : null;
  const stick = useStickToBottomScroll(
    scrollRef,
    { historyLength, hasLiveRow },
    { enabled: true, watchCharacterData: false },
  );

  useLayoutEffect(() => {
    if (!stick.isPinned || liveTailText == null) return;
    const el = scrollRef.current;
    if (!el) return;
    scrollElementToBottom(el);
  }, [stick.isPinned, liveTailText]);

  return { scrollRef, ...stick };
}

const DirectionColumn = memo(function DirectionColumn({
  rows,
  fontScale,
  transcriptLayout,
  notesMode,
  direction,
  controls,
  interactive,
}: {
  rows: OverlayTailRow[];
  fontScale: number;
  transcriptLayout: TranscriptLayout;
  notesMode: boolean;
  direction: "outbound" | "inbound";
  controls?: OverlayControlsProps | null;
  interactive: boolean;
}) {
  const fontSize = `${12 * fontScale}px`;
  const { scrollRef, isPinned, newSinceUnpinned, jumpToBottom } =
    useOverlayColumnScroll(rows, notesMode);

  const onOutputModeChange = useCallback(
    (value: string) => {
      controls?.onOutputModeChange(direction, value);
    },
    [controls, direction],
  );

  return (
    <div className="relative flex h-full min-h-0 min-w-0 flex-1 basis-0 flex-col gap-1 overflow-hidden">
      {controls ? (
        <OverlayColumnControls
          direction={direction}
          status={controls.status}
          column={direction === "outbound" ? controls.outbound : controls.inbound}
          muted={direction === "outbound" ? controls.micMuted : controls.speakerMuted}
          interactive={interactive}
          toolbarValue={
            direction === "outbound"
              ? controls.outboundToolbarMode
              : controls.inboundMode
          }
          modeOptions={
            direction === "outbound"
              ? controls.outboundModeOptions
              : controls.inboundModeOptions
          }
          customActive={direction === "outbound" ? controls.customActive : false}
          onMuteToggle={
            direction === "outbound"
              ? controls.onMicToggle
              : controls.onSpeakerToggle
          }
          onPathChange={
            direction === "outbound"
              ? controls.onOutboundPath
              : controls.onInboundPath
          }
          onOutputModeChange={onOutputModeChange}
          sessionMode={controls.sessionMode}
        />
      ) : null}
      <div
        ref={scrollRef}
        className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pr-0.5"
      >
        <div className="space-y-2">
          {rows.length === 0 ? (
            <p className="text-xs text-muted-foreground" style={{ fontSize }}>
              {notesMode ? "Waiting for speech…" : "…"}
            </p>
          ) : (
            rows.map((row) => {
              const notesText = notesSegmentText(
                row.sourceText,
                row.translatedText,
              );
              return (
                <div
                  key={row.id}
                  className={cn(
                    "group/row relative rounded-sm pl-1.5",
                    row.live && "border-l-2 border-accent",
                  )}
                  style={{ fontSize }}
                >
                  {notesMode ? (
                    <p className="leading-snug text-foreground">{notesText}</p>
                  ) : transcriptLayout === "stacked" ? (
                    <div className="space-y-0.5">
                      <p className="leading-snug text-muted-foreground">
                        {row.sourceText}
                      </p>
                      <p className="leading-snug text-foreground">
                        {row.translatedText}
                      </p>
                    </div>
                  ) : (
                    <div className="grid grid-cols-2 gap-1.5">
                      <p className="leading-snug text-muted-foreground">
                        {row.sourceText}
                      </p>
                      <p className="leading-snug text-foreground">
                        {row.translatedText}
                      </p>
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
      {!isPinned && rows.length > 0 ? (
        <AppTooltip label="Jump to live">
          <Button
            type="button"
            size="sm"
            variant="secondary"
            className={cn(
              "absolute bottom-1 left-1/2 z-[3] h-7 -translate-x-1/2 gap-1 px-2 shadow-md",
              "border border-border bg-card/95",
            )}
            data-tauri-drag-region="false"
            onClick={(e) => {
              e.stopPropagation();
              jumpToBottom();
            }}
          >
            <ArrowDown className="size-3.5" aria-hidden strokeWidth={2} />
            {newSinceUnpinned > 0 ? (
              <span className="text-[10px] font-medium tabular-nums">
                {newSinceUnpinned > 9 ? "9+" : newSinceUnpinned}
              </span>
            ) : null}
          </Button>
        </AppTooltip>
      ) : null}
    </div>
  );
});

/** Notes overlay: single Direct|Notes path control (both directions). */
const NotesOverlayPathBar = memo(function NotesOverlayPathBar({
  controls,
  interactive,
  showYou,
  showMeeting,
}: {
  controls: OverlayControlsProps;
  interactive: boolean;
  showYou: boolean;
  showMeeting: boolean;
}) {
  const outbound = controls.outbound;
  const inbound = controls.inbound;
  const notesActive =
    outbound.pathMode === "translate" || inbound.pathMode === "translate";
  const directActive =
    !notesActive &&
    ((outbound.pathMode === "direct" &&
      (outbound.canDirect || outbound.column?.pipeline === "direct")) ||
      (inbound.pathMode === "direct" &&
        (inbound.canDirect || inbound.column?.pipeline === "direct")));

  const directDisabled =
    !interactive || outbound.directDisabled || inbound.directDisabled;
  const notesDisabled =
    !interactive || outbound.translateDisabled || inbound.translateDisabled;

  const onPathChange = (mode: AudioPathMode) => {
    if (mode === "direct" && directActive) return;
    if (mode === "translate" && notesActive) return;
    controls.onOutboundPath(mode);
    controls.onInboundPath(mode);
  };

  return (
    <div className="flex min-w-0 shrink-0 items-center gap-1.5 pb-1">
      <div className="flex min-w-0 flex-1 items-center justify-start">
        {showYou ? (
          <div className="shrink-0 [&_button]:!size-7 [&_svg]:!size-3.5">
            <MicMuteButton
              muted={controls.micMuted}
              disabled={!interactive || !outbound.muteEnabled}
              onToggle={controls.onMicToggle}
            />
          </div>
        ) : null}
      </div>

      <div
        className="inline-flex h-7 shrink-0 items-center gap-0.5 rounded-lg bg-secondary/90 p-0.5"
        role="group"
        aria-label="Notes audio path"
      >
        <AppTooltip
          label={
            outbound.awaitingDirectStandby || inbound.awaitingDirectStandby
              ? "Starting direct audio…"
              : outbound.canDirect && inbound.canDirect
                ? "Direct — passthrough only, not capturing notes (no STT)"
                : "Configure audio devices in Settings"
          }
        >
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            className={cn(
              "size-6 rounded-md border-0 shadow-none",
              directActive &&
                "bg-card text-[var(--pipeline-direct-text)] shadow-sm hover:bg-[var(--pipeline-direct-bg)] hover:shadow-md hover:text-[var(--pipeline-direct-text)] dark:bg-hover-surface",
            )}
            disabled={directDisabled}
            aria-pressed={directActive}
            aria-label="Direct"
            onClick={() => onPathChange("direct")}
          >
            <DirectPathIcon aria-hidden />
          </Button>
        </AppTooltip>
        <AppTooltip
          label={
            outbound.audioFault || inbound.audioFault
              ? "Audio device disconnected — wait for reconnect or plug the device back in"
              : outbound.canTranslate && inbound.canTranslate
                ? "Start notes — STT captions + natural audio (no translation)"
                : "Add API key and audio devices in Settings"
          }
        >
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            className={cn(
              "size-6 rounded-md border-0 shadow-none",
              notesActive
                ? "bg-[var(--pipeline-notes-bg)] text-[var(--pipeline-notes-text)] shadow-sm hover:bg-[var(--pipeline-notes-hover)]"
                : "text-muted-foreground hover:bg-card/70 hover:text-foreground",
            )}
            disabled={notesDisabled}
            aria-pressed={notesActive}
            aria-label="Notes"
            onClick={() => onPathChange("translate")}
          >
            <NotesPathIcon aria-hidden />
          </Button>
        </AppTooltip>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-end">
        {showMeeting ? (
          <div className="shrink-0 [&_button]:!size-7 [&_svg]:!size-3.5">
            <SpeakerMuteButton
              muted={controls.speakerMuted}
              disabled={!interactive || !inbound.muteEnabled}
              onToggle={controls.onSpeakerToggle}
            />
          </div>
        ) : null}
      </div>
    </div>
  );
});

export default function OverlayTranscriptView({
  rows,
  overlay,
  transcriptLayout,
  notesMode = false,
  title = "Meeting",
  onHide,
  compact = false,
  controls = null,
}: Props) {
  const visible = filterOverlayRows(rows, overlay.showInbound, overlay.showOutbound);
  const outbound = visible.filter((r) => r.direction === "outbound");
  const inbound = visible.filter((r) => r.direction === "inbound");
  const showYou = overlay.showOutbound;
  const showMeeting = overlay.showInbound;
  const opacityPct = Math.round(Math.min(1, Math.max(0, overlay.opacity)) * 100);
  const interactive = Boolean(controls) && !compact;
  const empty = visible.length === 0;
  const showResize = !compact && !overlay.clickThrough;

  const [size, setSize] = useState({
    width: overlay.width,
    height: overlay.height,
  });
  useEffect(() => {
    setSize({ width: overlay.width, height: overlay.height });
  }, [overlay.width, overlay.height]);

  const onHeaderMouseDown = (e: MouseEvent) => {
    if (compact) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("button")) return;
    void getCurrentWindow().startDragging();
  };

  return (
    <div
      className="relative flex h-full w-full flex-col overflow-hidden rounded-md border border-border/60 shadow-lg"
      style={{
        backgroundColor: `color-mix(in srgb, var(--card) ${opacityPct}%, transparent)`,
      }}
    >
      <div
        className="flex items-center gap-1 border-b border-border/50 px-2 py-1.5"
        {...(!compact ? { "data-tauri-drag-region": "" } : {})}
        onMouseDown={onHeaderMouseDown}
      >
        <GripVertical
          className="size-3.5 shrink-0 text-muted-foreground"
          {...(!compact ? { "data-tauri-drag-region": "" } : {})}
        />
        <span
          className="min-w-0 flex-1 truncate text-xs font-medium text-foreground"
          {...(!compact ? { "data-tauri-drag-region": "" } : {})}
        >
          {title}
        </span>
        {controls ? (
          <>
            {overlay.clickThrough ? (
              <span
                className="shrink-0 text-[10px] font-medium text-accent"
                {...(!compact ? { "data-tauri-drag-region": "" } : {})}
              >
                Click-through · hold {overlayHoldKey}
              </span>
            ) : null}
            <button
              type="button"
              className="cursor-pointer rounded p-1 text-muted-foreground hover:bg-hover-surface hover:text-foreground disabled:pointer-events-none disabled:opacity-50"
              aria-label={`Opacity ${opacityPct}%`}
              data-tauri-drag-region="false"
              disabled={!interactive}
              onClick={controls.onCycleOpacity}
            >
              <SunMedium className="size-3.5" />
            </button>
          </>
        ) : null}
        {onHide ? (
          <button
            type="button"
            className="cursor-pointer rounded p-1 text-muted-foreground hover:bg-hover-surface hover:text-foreground"
            aria-label="Close overlay"
            data-tauri-drag-region="false"
            onClick={onHide}
          >
            <X className="size-3.5" />
          </button>
        ) : null}
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden p-2">
        {notesMode && controls ? (
          <NotesOverlayPathBar
            controls={controls}
            interactive={interactive}
            showYou={showYou}
            showMeeting={showMeeting}
          />
        ) : null}
        <div
          className={cn(
            "min-h-0 flex-1 overflow-hidden",
            showYou && showMeeting ? "grid grid-cols-2 gap-0" : "flex",
          )}
        >
          {empty && !controls ? (
            <p className="m-auto text-xs text-muted-foreground">
              Waiting for speech…
            </p>
          ) : (
            <>
              {showYou ? (
                <div
                  className={cn(
                    "flex min-h-0 min-w-0 flex-col",
                    showYou && showMeeting && "border-r border-border/40 pr-2",
                    !(showYou && showMeeting) && "min-w-0 flex-1",
                  )}
                >
                  <DirectionColumn
                    rows={outbound}
                    fontScale={overlay.fontScale}
                    transcriptLayout={transcriptLayout}
                    notesMode={notesMode}
                    direction="outbound"
                    controls={notesMode ? null : controls}
                    interactive={interactive}
                  />
                </div>
              ) : null}
              {showMeeting ? (
                <div
                  className={cn(
                    "flex min-h-0 min-w-0 flex-col",
                    showYou && showMeeting && "pl-2",
                    !(showYou && showMeeting) && "min-w-0 flex-1",
                  )}
                >
                  <DirectionColumn
                    rows={inbound}
                    fontScale={overlay.fontScale}
                    transcriptLayout={transcriptLayout}
                    notesMode={notesMode}
                    direction="inbound"
                    controls={notesMode ? null : controls}
                    interactive={interactive}
                  />
                </div>
              ) : null}
            </>
          )}
        </div>
      </div>
      {empty && controls ? (
        <p className="px-2 pb-2 text-center text-[10px] text-muted-foreground">
          Waiting for speech…
        </p>
      ) : null}
      {showResize ? (
        <OverlayResizeHandle
          width={size.width}
          height={size.height}
          onSizeChange={(w, h) => setSize({ width: w, height: h })}
        />
      ) : null}
    </div>
  );
}
