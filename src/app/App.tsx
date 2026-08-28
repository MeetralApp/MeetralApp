import { useEffect, useRef } from "react";
import PlatformGuard from "@/features/pipeline/components/PlatformGuard";
import AppShell from "@/shared/layout/AppShell";
import LoadingSkeleton from "@/features/pipeline/components/LoadingSkeleton";
import ThemeConfigSync from "@/shared/components/ThemeConfigSync";
import { useActiveMeeting } from "@/features/meeting/library/hooks/useActiveMeeting";
import { useAppNavigation } from "@/shared/hooks/useAppNavigation";
import { endLiveMeetingSession } from "@/features/meeting/library/lib/endLiveMeetingSession";
import { endMeeting, renameMeeting } from "@/features/meeting/library/lib/meetingApi";
import { usePipelineRuntime } from "@/features/pipeline/hooks/usePipelineRuntime";
import { useToast } from "@/shared/context/useToast";
import { useTranscripts } from "@/features/pipeline/context/transcript/useLiveTranscript";
import { useMinDisplayDelay } from "@/shared/hooks/useMinDisplayDelay";
import OverlayApp from "@/features/overlay/OverlayApp";

function isOverlayWindow(): boolean {
  return window.location.hash.startsWith("#/overlay");
}

export default function App() {
  if (isOverlayWindow()) {
    return <OverlayApp />;
  }

  return <MainApp />;
}

function MainApp() {
  const { showToast } = useToast();
  const {
    config,
    setup,
    loading,
    setOutboundAudioMode,
    setInboundAudioMode,
  } = usePipelineRuntime();

  const { activeMeeting, refresh: refreshActiveMeeting } = useActiveMeeting();
  const { clearTranscripts } = useTranscripts();
  const nav = useAppNavigation();
  const prevLiveMeetingIdRef = useRef<string | null>(null);
  const skipAutoEndToastRef = useRef(false);

  // Engine auto-ends the live meeting (idle timeout or audio reconnect exhausted).
  // Mirror the manual End meeting toast + transcript clear (skip when user ended).
  useEffect(() => {
    const prevId = prevLiveMeetingIdRef.current;
    const nextId = activeMeeting?.id ?? null;
    prevLiveMeetingIdRef.current = nextId;
    if (!prevId || nextId) return;
    if (skipAutoEndToastRef.current) {
      skipAutoEndToastRef.current = false;
      return;
    }
    clearTranscripts();
    showToast("success", "Meeting saved");
  }, [activeMeeting, clearTranscripts, showToast]);

  const bootstrapping = loading || !config || !setup;
  const showBootSkeleton = useMinDisplayDelay(bootstrapping);

  if (showBootSkeleton || !config || !setup) {
    return (
      <PlatformGuard>
        <ThemeConfigSync preference={config?.themePreference} />
        <LoadingSkeleton />
      </PlatformGuard>
    );
  }

  const handleBackFromDetail = () => {
    const fromEndedMeeting = !(
      activeMeeting?.status === "live" &&
      nav.detailMeetingId === activeMeeting.id
    );
    nav.backToLive();
    if (fromEndedMeeting) {
      nav.closeSettings();
      nav.setLibraryOpen(true);
    }
  };

  const handleNewMeetingStarted = () => {
    clearTranscripts();
    nav.backToLive();
    void refreshActiveMeeting();
  };

  return (
    <PlatformGuard>
      <ThemeConfigSync preference={config.themePreference} />
      <AppShell
        appView={nav.appView}
        detailMeetingId={nav.detailMeetingId}
        pendingScrollSegment={nav.pendingScrollSegment}
        drawerOpen={nav.drawerOpen}
        libraryOpen={nav.libraryOpen}
        settingsFocus={nav.settingsFocus}
        activeMeeting={activeMeeting}
        onBackFromDetail={handleBackFromDetail}
        onOpenLibrary={nav.openLibrary}
        onOpenSettings={nav.openSettings}
        onCloseSettings={nav.closeSettings}
        onCloseLibrary={nav.closeLibrary}
        onOpenMeetingDetail={nav.openMeetingDetail}
        onOpenMeetingSearchHit={(hit) => {
          nav.openMeetingDetail(hit.meetingId, {
            meetingId: hit.meetingId,
            segmentId: hit.bestSegmentId,
          });
        }}
        onPendingScrollConsumed={nav.consumePendingScrollSegment}
        onBackToLive={nav.backToLive}
        onNewMeetingStarted={handleNewMeetingStarted}
        onRenameActiveMeeting={async (title) => {
          if (!activeMeeting) return;
          await renameMeeting(activeMeeting.id, title);
          await refreshActiveMeeting();
          showToast("success", "Meeting renamed");
        }}
        onEndActiveMeeting={async () => {
          if (!activeMeeting) return;
          skipAutoEndToastRef.current = true;
          // Best-effort stop paths — device-lost often makes Direct switch fail;
          // never block persisting the ended meeting on audio teardown.
          await endLiveMeetingSession({
            meetingId: activeMeeting.id,
            stopOutbound: () => setOutboundAudioMode("direct"),
            stopInbound: () => setInboundAudioMode("direct"),
            endMeeting,
          });
          clearTranscripts();
          await refreshActiveMeeting();
          showToast("success", "Meeting saved");
        }}
        onMeetingDeleted={() => {
          nav.backToLive();
          nav.setLibraryOpen(true);
          showToast("success", "Meeting deleted");
        }}
        onMeetingCreated={() => {
          showToast("success", "New meeting started");
        }}
      />
    </PlatformGuard>
  );
}
