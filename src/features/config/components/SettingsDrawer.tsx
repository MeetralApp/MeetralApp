import { useCallback, useEffect, useRef, useState } from "react";
import { X } from "lucide-react";

import ConfirmDialog from "@/shared/components/ConfirmDialog";
import { Button } from "@/shared/ui/button";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/shared/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/shared/ui/tabs";
import { useToast } from "@/shared/context/useToast";
import type { SettingsFocus, SettingsTab } from "../lib/settingsFocus";
import { resolveSettingsTab } from "../lib/settingsFocus";
import type { AppStatus, ConfigView, DevicesResponse, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { isPipelineBusy } from "@/features/pipeline/lib/pipelineStatus";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import type { AudioSetupValidation } from "@/features/audio/lib/audioSetup";
import {
  audioSectionStatus,
  audioTabWarn,
  dirtySectionStatus,
  intelligenceSectionStatus,
  intelligenceTabWarn,
  translateTabWarn,
  translationSectionStatus,
  voiceSectionStatus,
  voiceTabWarn,
} from "../lib/settingsDrawerStatus";
import {
  AppTabPanel,
  AudioTabPanel,
  IntelligenceTabPanel,
  TranslateTabPanel,
  VoiceTabPanel,
} from "./settingsDrawerTabs";

function isAlertDialogEventTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return Boolean(
    target.closest(
      '[data-slot="alert-dialog-overlay"], [data-slot="alert-dialog-content"], [role="alertdialog"]',
    ),
  );
}

interface Props {
  open: boolean;
  onClose: () => void;
  settingsFocus?: SettingsFocus | null;
  config: ConfigView;
  status: AppStatus | null;
  devices: DevicesResponse | null;
  audioSetup: AudioSetupValidation | null;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onSaveLanguages: (myLanguage: string, meetingLanguage: string) => Promise<void>;
  onTestApiKey: (
    apiKey: string,
    provider?: AiProvider,
    purpose?: "live" | "summary",
  ) => Promise<void>;
  onRefreshDevices: () => Promise<DevicesResponse | void>;
}

function TabWarnDot({ show }: { show: boolean }) {
  if (!show) return null;
  return (
    <span
      className="size-1.5 shrink-0 rounded-full bg-warning"
      aria-hidden
    />
  );
}

export default function SettingsDrawer({
  open,
  onClose,
  settingsFocus,
  config,
  status,
  devices,
  audioSetup,
  onSave,
  onSaveLanguages,
  onTestApiKey,
  onRefreshDevices,
}: Props) {
  const { showToast } = useToast();
  const languagesLocked = isPipelineBusy(status);
  const outboundLocked =
    status?.outbound === "active" ||
    status?.outbound === "starting" ||
    status?.outbound === "stopping";
  const inboundLocked =
    status?.inbound === "active" ||
    status?.inbound === "starting" ||
    status?.inbound === "stopping";

  const [activeTab, setActiveTab] = useState<SettingsTab>("translate");
  const [internalFocus, setInternalFocus] = useState<SettingsFocus | null>(null);
  const [audioDirty, setAudioDirty] = useState(false);
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [intelligenceApiKeyDirty, setIntelligenceApiKeyDirty] = useState(false);
  const [voiceDirty, setVoiceDirty] = useState(false);
  const [appDirty, setAppDirty] = useState(false);
  const [sonioxContextDirty, setSonioxContextDirty] = useState(false);
  const [speechDetectionDirty, setSpeechDetectionDirty] = useState(false);
  const [discardOpen, setDiscardOpen] = useState(false);
  const [contentKey, setContentKey] = useState(0);
  /** After "Keep editing", Sheet still sees the same outside-click — ignore briefly. */
  const suppressDiscardRef = useRef(false);
  const suppressTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const armDiscardSuppress = useCallback(() => {
    suppressDiscardRef.current = true;
    if (suppressTimerRef.current != null) {
      clearTimeout(suppressTimerRef.current);
    }
    suppressTimerRef.current = setTimeout(() => {
      suppressDiscardRef.current = false;
      suppressTimerRef.current = null;
    }, 300);
  }, []);

  useEffect(() => {
    return () => {
      if (suppressTimerRef.current != null) {
        clearTimeout(suppressTimerRef.current);
      }
    };
  }, []);

  const effectiveFocus = internalFocus ?? settingsFocus;
  const voiceSectionFocus =
    effectiveFocus === "clone" ? "voice" : effectiveFocus;

  const hasUnsavedChanges =
    apiKeyDirty ||
    intelligenceApiKeyDirty ||
    voiceDirty ||
    audioDirty ||
    appDirty ||
    sonioxContextDirty ||
    speechDetectionDirty;

  const clearDirtyFlags = useCallback(() => {
    setAudioDirty(false);
    setApiKeyDirty(false);
    setIntelligenceApiKeyDirty(false);
    setVoiceDirty(false);
    setAppDirty(false);
    setSonioxContextDirty(false);
    setSpeechDetectionDirty(false);
    setInternalFocus(null);
  }, []);

  const wipeDraftsAndClose = useCallback(() => {
    setDiscardOpen(false);
    clearDirtyFlags();
    onClose();
  }, [clearDirtyFlags, onClose]);

  const requestClose = useCallback(() => {
    if (suppressDiscardRef.current) return;
    if (hasUnsavedChanges) {
      setDiscardOpen(true);
      return;
    }
    onClose();
  }, [hasUnsavedChanges, onClose]);

  const onDiscardOpenChange = useCallback(
    (next: boolean) => {
      if (!next) {
        // Keep editing / Escape / overlay click on the confirm — do not let the
        // Sheet outside-dismiss immediately reopen this dialog.
        armDiscardSuppress();
      }
      setDiscardOpen(next);
    },
    [armDiscardSuppress],
  );

  const onSheetOpenChange = useCallback(
    (isOpen: boolean) => {
      if (isOpen) return;
      if (discardOpen || suppressDiscardRef.current) return;
      requestClose();
    },
    [discardOpen, requestClose],
  );

  const blockSheetDismissWhileDirty = useCallback(
    (event: { preventDefault: () => void; target: EventTarget | null }) => {
      if (isAlertDialogEventTarget(event.target)) {
        event.preventDefault();
        return;
      }
      if (discardOpen || suppressDiscardRef.current) {
        event.preventDefault();
        return;
      }
      if (!hasUnsavedChanges) return;
      event.preventDefault();
      setDiscardOpen(true);
    },
    [discardOpen, hasUnsavedChanges],
  );

  const translateWarn = translateTabWarn(
    config,
    apiKeyDirty,
    sonioxContextDirty,
    speechDetectionDirty,
  );
  const intelligenceWarn = intelligenceTabWarn(config);
  const voiceWarn = voiceTabWarn(config, voiceDirty);
  const audioWarn = audioTabWarn(audioDirty, audioSetup);
  const appWarn = appDirty;

  const translationStatus = translationSectionStatus(config, apiKeyDirty);
  const intelligenceStatus = intelligenceSectionStatus(config);
  const audioStatus = audioSectionStatus(audioDirty, audioSetup);
  const voiceStatus = voiceSectionStatus(config, voiceDirty);
  const appStatus = dirtySectionStatus(appDirty);
  const sonioxContextStatus = dirtySectionStatus(sonioxContextDirty);

  useEffect(() => {
    if (!open) {
      setDiscardOpen(false);
      clearDirtyFlags();
      return;
    }
    setContentKey((k) => k + 1);
  }, [open, clearDirtyFlags]);

  useEffect(() => {
    if (!open) return;
    const tab = resolveSettingsTab(settingsFocus);
    setActiveTab(tab);
    setInternalFocus(null);
  }, [open, settingsFocus]);

  return (
    <>
    <Sheet open={open} onOpenChange={onSheetOpenChange}>
      <SheetContent
        side="right"
        showCloseButton={false}
        className="flex w-[440px] max-w-[92vw] flex-col gap-0 p-0 sm:max-w-[440px]"
        aria-label="Settings"
        onPointerDownOutside={blockSheetDismissWhileDirty}
        onInteractOutside={blockSheetDismissWhileDirty}
        onEscapeKeyDown={blockSheetDismissWhileDirty}
      >
        <SheetHeader className="flex-row items-center justify-between space-y-0 border-b px-3 py-2">
          <div className="flex min-w-0 items-center gap-2">
            <SheetTitle asChild>
              <h2 className="m-0 text-base font-semibold text-foreground">
                Settings
              </h2>
            </SheetTitle>
            {hasUnsavedChanges ? (
              <span className="text-xs font-medium text-muted-foreground">
                Unsaved
              </span>
            ) : null}
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Close settings"
            onClick={requestClose}
          >
            <X className="size-4" aria-hidden strokeWidth={2} />
          </Button>
        </SheetHeader>

        <Tabs
          key={contentKey}
          value={activeTab}
          onValueChange={(value) => {
            setActiveTab(value as SettingsTab);
            setInternalFocus(null);
          }}
          className="flex min-h-0 flex-1 flex-col gap-0"
        >
          <div className="border-b px-3 pt-2">
            <TabsList
              variant="line"
              className="h-auto w-full justify-stretch gap-0"
            >
              <TabsTrigger value="translate" className="flex-1 px-1 text-[11px] sm:px-1.5 sm:text-sm">
                Translate
                <TabWarnDot show={translateWarn} />
              </TabsTrigger>
              <TabsTrigger value="voice" className="flex-1 px-1 text-[11px] sm:px-1.5 sm:text-sm">
                Voice
                <TabWarnDot show={voiceWarn} />
              </TabsTrigger>
              <TabsTrigger value="audio" className="flex-1 px-1 text-[11px] sm:px-1.5 sm:text-sm">
                Audio
                <TabWarnDot show={audioWarn} />
              </TabsTrigger>
              <TabsTrigger value="intelligence" className="flex-1 px-1 text-[11px] sm:px-1.5 sm:text-sm">
                Intelligence
                <TabWarnDot show={intelligenceWarn} />
              </TabsTrigger>
              <TabsTrigger value="app" className="flex-1 px-1 text-[11px] sm:px-1.5 sm:text-sm">
                App
                <TabWarnDot show={appWarn} />
              </TabsTrigger>
            </TabsList>
          </div>

          <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-[18px] py-2 pb-6">
            <TabsContent value="translate" className="mt-0">
              <TranslateTabPanel
                config={config}
                languagesLocked={languagesLocked}
                effectiveFocus={effectiveFocus}
                translationStatus={translationStatus}
                sonioxContextStatus={sonioxContextStatus}
                onSave={onSave}
                onSaveLanguages={onSaveLanguages}
                onTestApiKey={onTestApiKey}
                onToast={showToast}
                onApiKeyDirty={setApiKeyDirty}
                onSonioxContextDirty={setSonioxContextDirty}
                onSpeechDetectionDirty={setSpeechDetectionDirty}
              />
            </TabsContent>

            <TabsContent value="voice" className="mt-0">
              <VoiceTabPanel
                config={config}
                outboundLocked={outboundLocked}
                inboundLocked={inboundLocked}
                voiceStatus={voiceStatus}
                voiceSectionFocus={voiceSectionFocus}
                effectiveFocus={effectiveFocus}
                onSave={onSave}
                onToast={showToast}
                onVoiceDirty={setVoiceDirty}
              />
            </TabsContent>

            <TabsContent value="audio" className="mt-0">
              <AudioTabPanel
                config={config}
                devices={devices}
                audioSetup={audioSetup}
                audioStatus={audioStatus}
                effectiveFocus={effectiveFocus}
                onSave={onSave}
                onRefreshDevices={onRefreshDevices}
                onToast={showToast}
                onAudioDirty={setAudioDirty}
              />
            </TabsContent>

            <TabsContent value="intelligence" className="mt-0">
              <IntelligenceTabPanel
                config={config}
                intelligenceStatus={intelligenceStatus}
                effectiveFocus={effectiveFocus}
                onSave={onSave}
                onTestApiKey={onTestApiKey}
                onToast={showToast}
                onApiKeyDirty={setIntelligenceApiKeyDirty}
              />
            </TabsContent>

            <TabsContent value="app" className="mt-0">
              <AppTabPanel
                config={config}
                appStatus={appStatus}
                effectiveFocus={effectiveFocus}
                onSave={onSave}
                onToast={showToast}
                onAppDirty={setAppDirty}
              />
            </TabsContent>
          </div>
        </Tabs>
      </SheetContent>
    </Sheet>

      <ConfirmDialog
        open={discardOpen}
        onOpenChange={onDiscardOpenChange}
        title="Discard unsaved changes?"
        description="Your edits in Settings will be lost."
        confirmLabel="Discard"
        cancelLabel="Keep editing"
        destructive
        onConfirm={wipeDraftsAndClose}
      />
    </>
  );
}
