import {
  memo,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";
import {
  AlertTriangle,
  ChevronDown,
  CircleCheck,
  Loader2,
  RefreshCw,
  UserRound,
} from "lucide-react";
import MicMuteButton from "@/features/audio/components/MicMuteButton";
import SpeakerMuteButton from "@/features/audio/components/SpeakerMuteButton";
import { usePipelineSessionElapsed } from "@/features/pipeline/hooks/usePipelineSessionElapsed";
import type { PipelineModeOption } from "@/features/pipeline/lib/pipelineLabels";
import {
  DirectPathIcon,
  NotesPathIcon,
  TranslatePathIcon,
  pipelineOutputModeIcon,
} from "@/features/pipeline/lib/pipelineModeIcons";
import type { AppStatus, AudioPathMode } from "@/shared/lib/types/pipeline";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import type { OverlayColumnRuntime } from "../lib/overlayRuntimeTypes";
import type { VoiceTtsStatusPayload } from "@/features/voice/lib/voiceTypes";
import AppTooltip from "@/shared/components/AppTooltip";
import { overlaySetPointerInteractive } from "../lib/overlayApi";
import { isNotesSession } from "@/features/pipeline/lib/sessionMode";
import type { ConfigView } from "@/shared/lib/types/pipeline";

const isOverlayWindow = () => getCurrentWindow().label === "overlay";

async function setOverlayPointerInteractive(on: boolean) {
  if (!isOverlayWindow()) return;
  try {
    await overlaySetPointerInteractive(on);
  } catch {
  /* ignore */
  }
}

type Props = {
  direction: "outbound" | "inbound";
  status: AppStatus | null;
  column: OverlayColumnRuntime;
  muted: boolean;
  interactive: boolean;
  toolbarValue: string;
  modeOptions: PipelineModeOption[];
  customActive?: boolean;
  sessionMode?: ConfigView["sessionMode"];
  onMuteToggle: () => void;
  onPathChange: (mode: AudioPathMode) => void | Promise<void>;
  onOutputModeChange: (value: string) => void | Promise<void>;
};

function useCustomVoiceDegraded(enabled: boolean): string | null {
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled) {
      setMessage(null);
      return;
    }
    return listenSafe<VoiceTtsStatusPayload>(APP_EVENTS.voiceTtsStatus, (event) => {
      if (event.payload.kind === "ready") {
        setMessage(null);
      } else {
        setMessage(event.payload.message);
      }
    });
  }, [enabled]);

  return message;
}

function StatusChip({
  status,
  direction,
  column,
  customDegraded,
  customActive,
}: {
  status: AppStatus | null;
  direction: "outbound" | "inbound";
  column: OverlayColumnRuntime;
  customDegraded: string | null;
  customActive: boolean;
}) {
  const timer = usePipelineSessionElapsed(status, direction, {
    tickWhenActive: true,
  });

  let chip: ReactNode = null;
  let tip: string | null = null;
  let tone = "text-muted-foreground border-border/50 bg-secondary/30";

  if (column.audioFault && column.idleLabel) {
    chip = <AlertTriangle className="size-3.5" aria-hidden />;
    tip = column.idleTitle ?? column.idleLabel;
    tone = "text-destructive border-destructive/40 bg-destructive/10";
  } else if (customDegraded) {
    chip = <AlertTriangle className="size-3.5" aria-hidden />;
    tip = customDegraded;
    tone = "text-destructive border-destructive/40 bg-destructive/10";
  } else if (timer.variant === "starting") {
    chip = <Loader2 className="size-3.5 animate-spin" aria-hidden />;
    tip = timer.display ?? "Starting…";
    tone = "text-muted-foreground border-border/50 bg-secondary/30";
  } else if (timer.variant === "stopping") {
    chip = <Loader2 className="size-3.5 animate-spin" aria-hidden />;
    tip = timer.display ?? "Stopping…";
    tone = "text-muted-foreground border-border/50 bg-secondary/30";
  } else if (timer.variant === "reconnecting") {
    chip = <RefreshCw className="size-3.5 animate-spin" aria-hidden />;
    tip = timer.title ?? "Reconnecting…";
    tone = "text-warning border-warning/40 bg-warning/10";
  } else if (timer.variant === "elapsed" && customActive) {
    chip = <UserRound className="size-3.5" aria-hidden />;
    tip = "Custom voice active";
    tone =
      "text-[var(--pipeline-translate-text)] border-[var(--pipeline-translate-text)]/30 bg-[var(--pipeline-translate-bg)]";
  } else if (column.idleLabel) {
    tip = column.idleTitle ?? column.idleLabel;
    const kind = column.idleKind ?? "checking";
    if (kind === "ready") {
      chip = <CircleCheck className="size-3.5" aria-hidden />;
      tone = "text-success border-[var(--pipeline-direct-border)]/50 bg-[var(--pipeline-direct-bg)]";
    } else if (kind === "checking") {
      chip = <Loader2 className="size-3.5 animate-spin" aria-hidden />;
      tone = "text-muted-foreground border-border/50 bg-secondary/30";
    } else if (kind === "audio-reconnecting") {
      chip = <RefreshCw className="size-3.5 animate-spin" aria-hidden />;
      tone = "text-warning border-warning/40 bg-warning/10";
    } else if (kind === "audio-lost" || kind === "error") {
      chip = <AlertTriangle className="size-3.5" aria-hidden />;
      tone = "text-destructive border-destructive/40 bg-destructive/10";
    } else {
      // setup | api-key | …
      chip = <AlertTriangle className="size-3.5" aria-hidden />;
      tone = "text-warning border-warning/40 bg-warning/10";
    }
  }

  if (!chip) return null;

  const body = (
    <span
      className={cn(
        "inline-flex size-7 shrink-0 items-center justify-center rounded-md border",
        tone,
      )}
    >
      {chip}
    </span>
  );

  if (!tip) return body;

  return (
    <AppTooltip label={tip}>
      <button type="button" className="cursor-default border-0 bg-transparent p-0">
        {body}
      </button>
    </AppTooltip>
  );
}

const segmentBtn =
  "size-6 shrink-0 rounded-md border-0 bg-transparent p-0 text-muted-foreground shadow-none transition-[background-color,box-shadow,color,opacity] duration-150 ease-out hover:bg-card/70 hover:text-foreground hover:shadow-sm disabled:opacity-50 dark:hover:bg-hover-surface/70 [&_svg]:size-3.5";

const OverlayColumnControls = memo(function OverlayColumnControls({
  direction,
  status,
  column,
  muted,
  interactive,
  toolbarValue,
  modeOptions,
  customActive = false,
  sessionMode,
  onMuteToggle,
  onPathChange,
  onOutputModeChange,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuPos, setMenuPos] = useState<{ top: number; left: number } | null>(
    null,
  );
  const notesMode = isNotesSession({ sessionMode });
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const pathMode = column.pathMode;
  const translateActive = pathMode === "translate";
  const directActive =
    pathMode === "direct" &&
    (column.canDirect || column.column?.pipeline === "direct");
  const customListening =
    interactive && direction === "outbound" && customActive && translateActive;
  const customDegraded = useCustomVoiceDegraded(customListening);

  const muteDisabled = !interactive || !column.muteEnabled;
  const directDisabled = !interactive || column.directDisabled;
  const translateDisabled = !interactive || column.translateDisabled;
  const chevronDisabled = translateDisabled || !interactive;

  const activeOption = modeOptions.find((o) => o.value === toolbarValue);
  const ModeIcon = pipelineOutputModeIcon(toolbarValue);
  const modeTip = activeOption?.title ?? activeOption?.label ?? "Output mode";

  const directTitle = column.awaitingDirectStandby
    ? "Starting direct audio…"
    : column.canDirect
      ? "Direct — natural audio, not recording (no API)"
      : "Configure audio devices in Settings";
  const translateTitle = column.audioFault
    ? "Audio device disconnected — wait for reconnect or plug the device back in"
    : column.canTranslate
      ? notesMode
        ? "Start notes — STT captions + natural audio (no translation)"
        : "Translate"
      : "Add API key and audio devices in Settings";
  const pathLabel = notesMode ? "Notes" : "Translate";
  const pathAria = notesMode ? "Notes" : "Translate";
  const PathIcon = notesMode ? NotesPathIcon : TranslatePathIcon;
  const pathActiveTone = notesMode
    ? "bg-[var(--pipeline-notes-bg)] text-[var(--pipeline-notes-text)] shadow-sm hover:bg-[var(--pipeline-notes-hover)]"
    : "bg-card text-[var(--pipeline-translate-text)] shadow-sm hover:bg-[var(--pipeline-translate-bg)] hover:shadow-md dark:bg-hover-surface";
  const pathActiveText = notesMode
    ? "text-[var(--pipeline-notes-text)] hover:text-[var(--pipeline-notes-text)]"
    : "text-[var(--pipeline-translate-text)] hover:text-[var(--pipeline-translate-text)]";

  const closeMenu = () => {
    setMenuOpen(false);
    setMenuPos(null);
    void setOverlayPointerInteractive(false);
  };

  const openMenu = async () => {
    if (chevronDisabled) return;
    // Pin interactive before paint — click-through On + hold modifier must not race.
    await setOverlayPointerInteractive(true);
    const rect = triggerRef.current?.getBoundingClientRect();
    if (rect) {
      setMenuPos({ top: rect.bottom + 4, left: rect.left });
    }
    setMenuOpen(true);
  };

  useLayoutEffect(() => {
    if (!menuOpen || !triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    setMenuPos({ top: rect.bottom + 4, left: rect.left });
  }, [menuOpen]);

  useEffect(() => {
    if (!menuOpen) return;

    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (menuRef.current?.contains(target)) return;
      if (triggerRef.current?.contains(target)) return;
      closeMenu();
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeMenu();
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [menuOpen]);

  useEffect(() => {
    return () => {
      void setOverlayPointerInteractive(false);
    };
  }, []);

  const handleOutputSelect = async (value: string) => {
    if (modeOptions.find((o) => o.value === value)?.disabled) return;
    closeMenu();
    await onOutputModeChange(value);
    if (!translateActive && column.canTranslate && !column.audioFault) {
      await onPathChange("translate");
    }
  };

  return (
    <div className="flex min-w-0 items-center gap-1.5">
      <div className="shrink-0 [&_button]:!size-7 [&_svg]:!size-3.5">
        {direction === "outbound" ? (
          <MicMuteButton
            muted={muted}
            disabled={muteDisabled}
            onToggle={onMuteToggle}
          />
        ) : (
          <SpeakerMuteButton
            muted={muted}
            disabled={muteDisabled}
            onToggle={onMuteToggle}
          />
        )}
      </div>

      <div
        className="inline-flex h-7 shrink-0 items-center gap-0.5 rounded-lg bg-secondary/90 p-0.5"
        role="group"
        aria-label={`${direction} audio path`}
      >
        <AppTooltip label={directTitle}>
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            className={cn(
              segmentBtn,
              "rounded-md border-0 shadow-none",
              directActive &&
                "bg-card text-[var(--pipeline-direct-text)] shadow-sm hover:bg-[var(--pipeline-direct-bg)] hover:shadow-md hover:text-[var(--pipeline-direct-text)] dark:bg-hover-surface",
            )}
            disabled={directDisabled}
            aria-pressed={directActive}
            aria-label="Direct"
            onClick={() => {
              closeMenu();
              if (!directActive) void onPathChange("direct");
            }}
          >
            <DirectPathIcon aria-hidden />
          </Button>
        </AppTooltip>

        <div
          className={cn(
            "inline-flex h-6 items-center rounded-md transition-[background-color,box-shadow,color] duration-150 ease-out",
            translateActive
              ? pathActiveTone
              : "text-muted-foreground hover:bg-card/70 hover:text-foreground hover:shadow-sm dark:hover:bg-hover-surface/70",
          )}
        >
          <AppTooltip label={translateTitle}>
            <Button
              type="button"
              size="icon-xs"
              variant="ghost"
              className={cn(
                segmentBtn,
                "border-0 shadow-none hover:bg-transparent",
                translateActive && pathActiveText,
              )}
              disabled={translateDisabled}
              aria-pressed={translateActive}
              aria-label={pathAria}
              title={pathLabel}
              onClick={() => {
                if (!translateActive) void onPathChange("translate");
              }}
            >
              <PathIcon aria-hidden />
            </Button>
          </AppTooltip>
          <AppTooltip label={modeTip}>
            <span
              className={cn(
                "inline-flex size-6 items-center justify-center [&_svg]:size-3.5",
                translateActive && pathActiveText,
              )}
            >
              <ModeIcon aria-hidden />
            </span>
          </AppTooltip>
          <Button
            ref={triggerRef}
            type="button"
            size="icon-xs"
            variant="ghost"
            className={cn(
              segmentBtn,
              "border-0 opacity-80 shadow-none hover:bg-transparent hover:opacity-100 [&_svg]:size-3",
              translateActive && pathActiveText,
              menuOpen && "opacity-100",
            )}
            disabled={chevronDisabled}
            aria-label="Output mode"
            aria-expanded={menuOpen}
            aria-haspopup="listbox"
            onPointerDown={(e) => {
              // Capture interactive mode on the same gesture as open (click-through).
              if (e.button !== 0 || chevronDisabled) return;
              void setOverlayPointerInteractive(true);
            }}
            onClick={(e) => {
              e.stopPropagation();
              if (menuOpen) closeMenu();
              else void openMenu();
            }}
          >
            <ChevronDown aria-hidden />
          </Button>
          {menuOpen && menuPos ? (
            <div
              ref={menuRef}
              role="listbox"
              aria-label="Output mode"
              className="fixed z-[200] min-w-[9.5rem] rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
              style={{ top: menuPos.top, left: menuPos.left }}
            >
              {modeOptions.map((opt) => {
                const Icon = pipelineOutputModeIcon(opt.value);
                const selected = opt.value === toolbarValue;
                const itemDisabled = opt.disabled || !interactive;
                const tip = opt.title ?? opt.label;
                return (
                  <AppTooltip key={opt.value} label={tip} side="left">
                    <span className="flex w-full">
                      <button
                        type="button"
                        role="option"
                        aria-selected={selected}
                        disabled={itemDisabled}
                        className={cn(
                          "flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs font-medium outline-none",
                          "hover:bg-hover-surface focus-visible:bg-hover-surface",
                          "disabled:cursor-not-allowed disabled:opacity-50",
                          selected && "bg-hover-surface",
                        )}
                        onClick={(e) => {
                          e.stopPropagation();
                          void handleOutputSelect(opt.value);
                        }}
                      >
                        <Icon
                          className="size-3.5 shrink-0 text-muted-foreground"
                          aria-hidden
                        />
                        <span>{opt.shortLabel ?? opt.label}</span>
                      </button>
                    </span>
                  </AppTooltip>
                );
              })}
            </div>
          ) : null}
        </div>
      </div>

      <div className="flex min-w-0 flex-1 justify-end">
        <StatusChip
          status={status}
          direction={direction}
          column={column}
          customDegraded={customDegraded}
          customActive={customActive && !customDegraded}
        />
      </div>
    </div>
  );
});

export default OverlayColumnControls;
