import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { guardUnlisten, listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";
import { useActiveMeeting } from "@/features/meeting/library/hooks/useActiveMeeting";
import { meetingChipDisplayTitle } from "@/features/meeting/library/lib/meetingDisplay";
import type { SegmentPreviewEvent } from "@/features/meeting/library/lib/meetingTypes";
import type { TranscriptEvent } from "@/shared/lib/types/pipeline";
import { createTranscriptCoalesceScheduler } from "@/features/pipeline/context/transcript/liveTranscriptCoalesce";
import { TooltipProvider } from "@/shared/ui/tooltip";
import { MOCK_OVERLAY_ROWS, type OverlayTailRow } from "./lib/mockTranscript";
import { liveOverlayRowId } from "./lib/overlayTail";
import {
  applyOverlayPendingBatch,
  OVERLAY_TRANSCRIPT_FLUSH_MS,
  type OverlayPending,
} from "./lib/overlayCoalesce";
import {
  overlayHide,
  overlayIsPreviewMode,
  overlayPatchSettings,
  overlaySetOffset,
} from "./lib/overlayApi";
import { shouldWipeOverlayTailOnPreviewEvent } from "./lib/overlayPreview";
import { useOverlayRuntime } from "./hooks/useOverlayRuntime";
import OverlayTranscriptView, {
  type OverlayControlsProps,
} from "./components/OverlayTranscriptView";
import ThemeConfigSync from "@/shared/components/ThemeConfigSync";

const OPACITY_STEPS = [0.7, 0.92, 1] as const;

type OverlayDirection = OverlayTailRow["direction"];

function asDirection(value: string): OverlayDirection {
  return value === "inbound" ? "inbound" : "outbound";
}

/** Live interim only — committed rows come from `segment-preview`. */
function transcriptEventToLiveRow(p: TranscriptEvent): OverlayTailRow | null {
  if (p.turnComplete || p.connectionGap) return null;
  if (p.interim !== true) return null;
  const direction = asDirection(p.direction);
  const sourceText = p.liveSource ?? p.sourceText ?? p.translatedText ?? "";
  const translatedText =
    p.liveTranslated ?? p.translatedText ?? p.sourceText ?? "";
  if (!sourceText && !translatedText) return null;
  return {
    id: liveOverlayRowId(direction),
    direction,
    sourceText,
    translatedText,
    live: true,
  };
}

function segmentPreviewToRow(p: SegmentPreviewEvent): OverlayTailRow | null {
  const sourceText = p.sourceText?.trim() ?? "";
  const translatedText = p.translatedText?.trim() ?? "";
  if (!sourceText && !translatedText) return null;
  const direction = asDirection(p.direction);
  return {
    id: `seg-${direction}-${p.sequence}`,
    direction,
    sourceText: p.sourceText,
    translatedText: p.translatedText,
    live: false,
  };
}

export default function OverlayApp() {
  const runtime = useOverlayRuntime();
  const { activeMeeting } = useActiveMeeting();
  const [rows, setRows] = useState<OverlayTailRow[]>([]);
  const [previewMode, setPreviewMode] = useState(false);
  const previewModeRef = useRef(false);
  const activeMeetingIdRef = useRef<string | null>(null);

  // Pending UI mutations coalesced into one React commit every OVERLAY_TRANSCRIPT_FLUSH_MS.
  const scheduler = useMemo(
    () =>
      createTranscriptCoalesceScheduler<OverlayPending>(
        (batch) => {
          setRows((prev) => applyOverlayPendingBatch(prev, batch));
        },
        OVERLAY_TRANSCRIPT_FLUSH_MS,
      ),
    [],
  );
  const prevMeetingIdRef = useRef<string | null | undefined>(undefined);

  activeMeetingIdRef.current = activeMeeting?.id ?? null;

  const overlay = runtime.overlay;
  const transcriptLayout = runtime.transcriptLayout;
  const meetingTitle = activeMeeting
    ? meetingChipDisplayTitle(activeMeeting)
    : previewMode
      ? "Preview"
      : "Meeting";

  const clearTranscriptRows = useCallback(() => {
    scheduler.clear();
    setRows([]);
  }, [scheduler]);

  const setPreviewModeSync = useCallback((on: boolean) => {
    previewModeRef.current = on;
    setPreviewMode(on);
  }, []);

  // Keep overlay tail in sync with Live: clear on end meeting / switch / new meeting.
  useEffect(() => {
    if (previewMode) return;
    const id = activeMeeting?.id ?? null;
    const prev = prevMeetingIdRef.current;
    if (prev === undefined) {
      prevMeetingIdRef.current = id;
      return;
    }
    if (prev === id) return;
    prevMeetingIdRef.current = id;
    clearTranscriptRows();
  }, [activeMeeting?.id, previewMode, clearTranscriptRows]);

  // Click-through: Rust owns ignore. ON = pass-through unless Ctrl (Windows)
  // / Cmd (macOS) held. Mode toggle: Settings / Tray / Ctrl|Cmd+Shift+T.

  useEffect(() => {
    document.documentElement.classList.add("overlay-window");
    document.body.classList.add("overlay-window");
    return () => {
      document.documentElement.classList.remove("overlay-window");
      document.body.classList.remove("overlay-window");
    };
  }, []);

  useEffect(
    () =>
      listenSafe(APP_EVENTS.overlayPassThrough, () => {
        // Pass-through ignores the cursor so Radix never gets pointerleave.
        // Blur active element instead of remounting TooltipProvider (that remount
        // rebuilt both columns + scroll observers and spiked CPU).
        const active = document.activeElement;
        if (active instanceof HTMLElement) active.blur();
      }),
    [],
  );

  useEffect(() => {
    let cancelled = false;

    const enqueue = (item: OverlayPending) => {
      scheduler.enqueue(item);
    };

    const leavePreviewForLiveSpeech = () => {
      if (!previewModeRef.current) return;
      setPreviewModeSync(false);
      clearTranscriptRows();
    };

    // Bootstrap: event may have fired before this webview mounted.
    void (async () => {
      try {
        if (await overlayIsPreviewMode()) {
          if (cancelled) return;
          setPreviewModeSync(true);
          setRows(MOCK_OVERLAY_ROWS);
        }
      } catch {
      /* ignore */
      }
    })();

    const unlistenTranscript = listenSafe<TranscriptEvent>(
      APP_EVENTS.transcript,
      (event) => {
        if (cancelled) return;
        const p = event.payload;
        const direction = asDirection(p.direction);

        // Turn/gap end: live→committed is owned by segment-preview; only clear live.
        if (p.turnComplete || p.connectionGap) {
          leavePreviewForLiveSpeech();
          enqueue({ kind: "clearLive", direction });
          return;
        }

        const row = transcriptEventToLiveRow(p);
        if (!row) return;
        leavePreviewForLiveSpeech();
        enqueue({ kind: "row", row });
      },
    );

    // Authoritative segment boundaries (sentence / turn / gap) — same as Live UI.
    const unlistenSegmentPreview = listenSafe<SegmentPreviewEvent>(
      APP_EVENTS.segmentPreview,
      (event) => {
        if (cancelled) return;
        const preview = event.payload;
        const meetingId = activeMeetingIdRef.current;
        if (meetingId && preview.meetingId !== meetingId) return;
        const row = segmentPreviewToRow(preview);
        if (!row) return;
        leavePreviewForLiveSpeech();
        enqueue({
          kind: "row",
          row,
          // Mid-turn sentence commits must keep the in-flight live tail.
          clearLive: preview.reason !== "sentence",
        });
      },
    );

    const unlistenPreviewMode = listenSafe<boolean>(
      APP_EVENTS.overlayPreviewMode,
      (event) => {
        if (cancelled) return;
        const on = event.payload === true;
        if (!shouldWipeOverlayTailOnPreviewEvent(previewModeRef.current, on)) {
          return;
        }
        setPreviewModeSync(on);
        clearTranscriptRows();
        if (on) {
          setRows(MOCK_OVERLAY_ROWS);
        }
      },
    );

    return () => {
      cancelled = true;
      unlistenTranscript();
      unlistenSegmentPreview();
      unlistenPreviewMode();
      scheduler.dispose();
    };
  }, [clearTranscriptRows, setPreviewModeSync, scheduler]);

  useEffect(() => {
    const win = getCurrentWindow();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unlisten = guardUnlisten(
      win.onMoved(async () => {
        clearTimeout(timer);
        timer = setTimeout(async () => {
          try {
            const pos = await win.outerPosition();
            await overlaySetOffset(pos.x, pos.y);
          } catch {
          /* ignore */
          }
        }, 250);
      }),
    );
    return () => {
      clearTimeout(timer);
      unlisten();
    };
  }, []);

  const onClose = useCallback(() => {
    // Persists Show transcript overlay = OFF (Settings + Tray stay in sync).
    void overlayHide();
  }, []);

  const onCycleOpacity = useCallback(() => {
    const current = overlay.opacity;
    const idx = OPACITY_STEPS.findIndex((v) => Math.abs(v - current) < 0.02);
    const next = OPACITY_STEPS[(idx + 1) % OPACITY_STEPS.length] ?? 0.92;
    void overlayPatchSettings({ opacity: next });
  }, [overlay.opacity]);

  const {
    status,
    outbound,
    inbound,
    micMuted,
    speakerMuted,
    outboundToolbarMode,
    inboundMode,
    outboundModeOptions,
    inboundModeOptions,
    customActive,
    toggleMicMuted,
    toggleSpeakerMuted,
    setOutboundPath,
    setInboundPath,
    setOutputMode,
  } = runtime;

  const controls = useMemo<OverlayControlsProps>(
    () => ({
      status,
      outbound,
      inbound,
      micMuted,
      speakerMuted,
      outboundToolbarMode,
      inboundMode,
      outboundModeOptions,
      inboundModeOptions,
      customActive,
      sessionMode: runtime.config?.sessionMode,
      onMicToggle: () => {
        void toggleMicMuted();
      },
      onSpeakerToggle: () => {
        void toggleSpeakerMuted();
      },
      onOutboundPath: (mode) => {
        void setOutboundPath(mode);
      },
      onInboundPath: (mode) => {
        void setInboundPath(mode);
      },
      onOutputModeChange: (direction, value) => {
        void setOutputMode(direction, value);
      },
      onCycleOpacity,
    }),
    [
      status,
      outbound,
      inbound,
      micMuted,
      speakerMuted,
      outboundToolbarMode,
      inboundMode,
      outboundModeOptions,
      inboundModeOptions,
      customActive,
      runtime.config?.sessionMode,
      toggleMicMuted,
      toggleSpeakerMuted,
      setOutboundPath,
      setInboundPath,
      setOutputMode,
      onCycleOpacity,
    ],
  );

  const displayRows = previewMode && rows.length === 0 ? MOCK_OVERLAY_ROWS : rows;
  const notesMode = runtime.config?.sessionMode === "notes";

  return (
    <div className="h-screen w-screen overflow-hidden bg-transparent">
      <ThemeConfigSync preference={runtime.config?.themePreference} />
      <TooltipProvider>
        <OverlayTranscriptView
          rows={displayRows}
          overlay={overlay}
          transcriptLayout={transcriptLayout}
          notesMode={notesMode}
          title={meetingTitle}
          onHide={onClose}
          controls={controls}
        />
      </TooltipProvider>
    </div>
  );
}
