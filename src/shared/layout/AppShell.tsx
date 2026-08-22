import { lazy, Suspense } from "react";
import HistoryButton from "@/features/meeting/library/components/HistoryButton";
import MeetingDetailHeaderTitle from "@/features/meeting/detail/components/MeetingDetailHeaderTitle";
import SessionHeaderHub from "@/features/pipeline/components/SessionHeaderHub";
import TranslateView from "@/features/pipeline/components/TranslateView";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import AppTooltip from "@/shared/components/AppTooltip";
import { useToast } from "@/shared/context/useToast";
import {
  formatSetupAttentionFromSetup,
  hasSetupIssuesFromSetup,
} from "@/features/pipeline/lib/appState";
import {
  getNewMeetingLock,
  isDirectionActive,
  isPipelineBusy,
} from "@/features/pipeline/lib/pipelineStatus";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import { usePipelineRuntime } from "@/features/pipeline/hooks/usePipelineRuntime";
import type { AppView } from "@/shared/lib/types/appView";
import type { SettingsFocus } from "@/features/config/lib/settingsFocus";
import type { MeetingRecord, PendingScrollSegment } from "@/features/meeting/library/lib/meetingTypes";
import { ArrowLeft, Settings } from "lucide-react";

// Lazy boundaries: heavy drawers/detail (TipTap editor deps) load on first open.
const MeetingDetailView = lazy(
  () => import("@/features/meeting/detail/components/MeetingDetailView"),
);
const LibraryDrawer = lazy(
  () => import("@/features/meeting/library/components/LibraryDrawer"),
);
const SettingsDrawer = lazy(
  () => import("@/features/config/components/SettingsDrawer"),
);

interface AppShellProps {
  appView: AppView;
  detailMeetingId: string | null;
  pendingScrollSegment: PendingScrollSegment | null;
  drawerOpen: boolean;
  libraryOpen: boolean;
  settingsFocus: SettingsFocus | null;
  activeMeeting: MeetingRecord | null;
  onBackFromDetail: () => void;
  onOpenLibrary: () => void;
  onOpenSettings: (focus?: SettingsFocus) => void;
  onCloseSettings: () => void;
  onCloseLibrary: () => void;
  onOpenMeetingDetail: (meetingId: string) => void;
  onOpenMeetingSearchHit: (hit: {
    meetingId: string;
    bestSegmentId: string;
  }) => void;
  onPendingScrollConsumed: () => void;
  onBackToLive: () => void;
  onNewMeetingStarted: () => void;
  onRenameActiveMeeting: (title: string) => Promise<void>;
  onEndActiveMeeting: () => Promise<void>;
  onMeetingDeleted: () => void;
  onMeetingCreated: () => void;
}

export default function AppShell({
  appView,
  detailMeetingId,
  pendingScrollSegment,
  drawerOpen,
  libraryOpen,
  settingsFocus,
  activeMeeting,
  onBackFromDetail,
  onOpenLibrary,
  onOpenSettings,
  onCloseSettings,
  onCloseLibrary,
  onOpenMeetingDetail,
  onOpenMeetingSearchHit,
  onPendingScrollConsumed,
  onBackToLive,
  onNewMeetingStarted,
  onRenameActiveMeeting,
  onEndActiveMeeting,
  onMeetingDeleted,
  onMeetingCreated,
}: AppShellProps) {
  const { showToast } = useToast();
  const {
    config,
    setup,
    status,
    error,
    setError,
    devices,
    columnUi,
    saveConfig,
    setOutboundOutputMode,
    setOutboundVoiceOutput,
    setInboundOutputMode,
    setInboundVoiceOutput,
    setOutboundAudioMode,
    setInboundAudioMode,
    setMicMuted,
    setSpeakerMuted,
    ensureDirectAudio,
    refreshDeviceCatalog,
    testApiKey,
  } = usePipelineRuntime();

  if (!config || !setup) {
    return null;
  }

  const audioSetup = setup.audio;
  const canDirectOut = setup.canDirectOutbound;
  const canDirectIn = setup.canDirectInbound;
  const canTranslateOut = setup.canTranslateOutbound;
  const canTranslateIn = setup.canTranslateInbound;
  const languagesLocked = isPipelineBusy(status);
  const newMeetingLock = getNewMeetingLock(
    status,
    config,
    canDirectOut,
    canDirectIn,
  );
  const setupAttention = hasSetupIssuesFromSetup(setup);
  const settingsLabel = setupAttention
    ? formatSetupAttentionFromSetup(setup)
    : "Settings";
  const outboundColumn = columnUi("outbound");
  const inboundColumn = columnUi("inbound");

  const saveLanguages = async (myLanguage: string, meetingLanguage: string) => {
    const notes = config.sessionMode === "notes";
    const nextMeeting = notes ? myLanguage : meetingLanguage;
    await saveConfig(
      toSavePayload({
        ...config,
        myLanguage,
        meetingLanguage: nextMeeting,
        ...(notes
          ? { notesLanguage: myLanguage }
          : {
              interpreterMyLanguage: myLanguage,
              interpreterMeetingLanguage: nextMeeting,
            }),
      }),
    );
    if (
      !notes &&
      config.sessionMode === "interpreter" &&
      myLanguage === nextMeeting
    ) {
      showToast(
        "info",
        "Same language on both sides — try Notes mode (Soniox or OpenAI) for STT capture without translation.",
      );
    }
  };

  const handleRefreshDevices = async () => {
    await refreshDeviceCatalog();
    await ensureDirectAudio();
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-2 p-3">
      <header className="grid shrink-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2">
        <div className="flex min-w-0 items-center gap-2 justify-self-start">
          {appView === "meeting-detail" ? (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="whitespace-nowrap"
              aria-label={
                config.sessionMode === "notes"
                  ? "Back to live"
                  : "Back to live translate"
              }
              onClick={onBackFromDetail}
            >
              <ArrowLeft size={16} aria-hidden strokeWidth={2} />
              Live
            </Button>
          ) : (
            <HistoryButton active={libraryOpen} onClick={onOpenLibrary} />
          )}
        </div>

        <div className="flex min-w-0 justify-center justify-self-stretch px-1">
          {appView === "live" ? (
            <SessionHeaderHub
              meeting={activeMeeting}
              myLanguage={config.myLanguage}
              meetingLanguage={config.meetingLanguage}
              languagesLocked={languagesLocked}
              showSonioxContext={config.aiProvider === "soniox"}
              config={config}
              contextDisabled={
                isDirectionActive(status, "outbound") ||
                isDirectionActive(status, "inbound")
              }
              onRename={onRenameActiveMeeting}
              onEnd={onEndActiveMeeting}
              onOpenLanguages={() => onOpenSettings("translate")}
              onSave={saveConfig}
              onToast={showToast}
            />
          ) : appView === "meeting-detail" && detailMeetingId ? (
            <MeetingDetailHeaderTitle
              meetingId={detailMeetingId}
              onDeleted={onMeetingDeleted}
            />
          ) : null}
        </div>

        <div className="flex min-w-0 items-center gap-2 justify-self-end">
          <AppTooltip label={settingsLabel}>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className={cn(
                "relative shrink-0",
                setupAttention &&
                  "after:absolute after:top-1.5 after:right-1.5 after:size-2 after:rounded-full after:bg-warning after:content-['']",
              )}
              onClick={() => onOpenSettings()}
              aria-label={settingsLabel}
            >
              <Settings size={20} aria-hidden strokeWidth={2} />
            </Button>
          </AppTooltip>
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col">
        {appView === "live" ? (
          <TranslateView
            config={config}
            status={status}
            error={error}
            setup={setup}
            outboundColumn={outboundColumn}
            inboundColumn={inboundColumn}
            canDirectOutbound={canDirectOut}
            canDirectInbound={canDirectIn}
            canTranslateOutbound={canTranslateOut}
            canTranslateInbound={canTranslateIn}
            onSaveConfig={saveConfig}
            onSetOutboundOutputMode={setOutboundOutputMode}
            onSetOutboundVoiceOutput={setOutboundVoiceOutput}
            onSetInboundOutputMode={setInboundOutputMode}
            onSetInboundVoiceOutput={setInboundVoiceOutput}
            onOutboundAudioPathChange={async (mode) => {
              setError(null);
              await setOutboundAudioMode(mode);
            }}
            onInboundAudioPathChange={async (mode) => {
              setError(null);
              await setInboundAudioMode(mode);
            }}
            onOpenSettings={onOpenSettings}
            onClearError={() => setError(null)}
            micMuted={status?.micMuted ?? false}
            onMicMuteToggle={() => {
              void setMicMuted(!(status?.micMuted ?? false));
            }}
            speakerMuted={status?.speakerMuted ?? false}
            onSpeakerMuteToggle={() => {
              void setSpeakerMuted(!(status?.speakerMuted ?? false));
            }}
          />
        ) : detailMeetingId ? (
          <Suspense
            fallback={
              <div className="flex min-h-0 flex-1 items-center justify-center text-sm text-muted-foreground">
                Loading meeting…
              </div>
            }
          >
            <MeetingDetailView
              meetingId={detailMeetingId}
              aiProvider={config.aiProvider}
              answerLanguage={config.answerLanguage ?? ""}
              artifactsEnabled={config.artifactsEnabled !== false}
              pendingScrollSegment={pendingScrollSegment}
              onPendingScrollConsumed={onPendingScrollConsumed}
              onReturnToLive={onBackToLive}
              onDeleted={onMeetingDeleted}
            />
          </Suspense>
        ) : null}
      </div>

      {libraryOpen ? (
        <Suspense fallback={null}>
          <LibraryDrawer
            open={libraryOpen}
            onClose={onCloseLibrary}
            onSelectMeeting={onOpenMeetingDetail}
            onSelectSearchHit={onOpenMeetingSearchHit}
            onNewMeetingStarted={onNewMeetingStarted}
            onReturnToLive={onBackToLive}
            onMeetingCreated={onMeetingCreated}
            activeMeetingId={activeMeeting?.id ?? null}
            newMeetingLock={newMeetingLock}
          />
        </Suspense>
      ) : null}

      {drawerOpen ? (
        <Suspense fallback={null}>
          <SettingsDrawer
            open={drawerOpen}
            onClose={onCloseSettings}
            settingsFocus={settingsFocus}
            config={config}
            status={status}
            devices={devices}
            audioSetup={audioSetup}
            onSave={async (payload) => {
              const result = await saveConfig(payload);
              await ensureDirectAudio();
              return result;
            }}
            onSaveLanguages={saveLanguages}
            onTestApiKey={testApiKey}
            onRefreshDevices={handleRefreshDevices}
          />
        </Suspense>
      ) : null}
    </div>
  );
}
