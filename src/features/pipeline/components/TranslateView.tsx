import SetupBanner from "@/features/audio/components/SetupBanner";
import PipelineErrorBanner from "@/features/audio/components/PipelineErrorBanner";
import {
  AUDIO_ROLE_FIELDS,
  emptyDeviceRef,
  isSetupBannerVisible,
  type PipelineErrorNotice,
} from "@/features/audio/lib/audioSetup";
import { useInboundTtsQueueDropToast } from "@/features/audio/hooks/useInboundTtsQueueDropToast";
import type { SettingsFocus } from "@/features/config/lib/settingsFocus";
import TranscriptPanel from "./TranscriptPanel";
import { useToast } from "@/shared/context/useToast";
import type { SetupState } from "../lib/appState";
import { columnUiFromLegacy, type ColumnUiState } from "../lib/columnUi";
import { toSavePayload } from "../lib/toSavePayload";
import type {
  AppStatus,
  AudioPathMode,
  ConfigView,
  InboundVoiceOutput,
  OutboundVoiceOutput,
  PipelineOutputMode,
  SaveConfigPayload,
  SaveConfigResult,
} from "@/shared/lib/types/pipeline";

interface Props {
  config: ConfigView;
  status: AppStatus | null;
  error: PipelineErrorNotice | null;
  setup: SetupState;
  outboundColumn: ColumnUiState | null;
  inboundColumn: ColumnUiState | null;
  canDirectOutbound: boolean;
  canDirectInbound: boolean;
  canTranslateOutbound: boolean;
  canTranslateInbound: boolean;
  onSaveConfig: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onSetOutboundOutputMode: (mode: PipelineOutputMode) => Promise<void>;
  onSetOutboundVoiceOutput: (voiceOutput: OutboundVoiceOutput) => Promise<void>;
  onSetInboundOutputMode: (mode: PipelineOutputMode) => Promise<void>;
  onSetInboundVoiceOutput: (voiceOutput: InboundVoiceOutput) => Promise<void>;
  onOutboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onInboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onOpenSettings: (focus?: SettingsFocus) => void;
  onClearError: () => void;
  micMuted?: boolean;
  onMicMuteToggle?: () => void;
  speakerMuted?: boolean;
  onSpeakerMuteToggle?: () => void;
}

export default function TranslateView(props: Props) {
  return <TranslateViewInner {...props} />;
}

function TranslateViewInner({
  config,
  status,
  error,
  setup,
  outboundColumn,
  inboundColumn,
  canDirectOutbound,
  canDirectInbound,
  canTranslateOutbound,
  canTranslateInbound,
  onSaveConfig,
  onSetOutboundOutputMode,
  onSetOutboundVoiceOutput,
  onSetInboundOutputMode,
  onSetInboundVoiceOutput,
  onOutboundAudioPathChange,
  onInboundAudioPathChange,
  onOpenSettings,
  onClearError,
  micMuted = false,
  onMicMuteToggle,
  speakerMuted = false,
  onSpeakerMuteToggle,
}: Props) {
  const { showToast } = useToast();
  useInboundTtsQueueDropToast();

  const outbound =
    outboundColumn ?? columnUiFromLegacy(setup, status, "outbound");
  const inbound =
    inboundColumn ?? columnUiFromLegacy(setup, status, "inbound");

  const handleLongReconnectRestored = (columnTitle: string) => {
    showToast("info", `${columnTitle} connection restored`);
  };

  const handleLongAudioReconnectRestored = (columnTitle: string) => {
    showToast("info", `${columnTitle} audio device restored`);
  };

  const handleUseSystemDefault = async () => {
    if (
      !error ||
      error.kind !== "device-unavailable" ||
      !error.role ||
      !error.allowSystemDefault
    ) {
      return;
    }
    const field = AUDIO_ROLE_FIELDS.find(
      (entry) => entry.role === error.role,
    )?.field;
    if (!field) return;

    await onSaveConfig(
      toSavePayload({
        ...config,
        [field]: emptyDeviceRef(),
      }),
    );
    onClearError();
  };

  const setupBannerVisible = isSetupBannerVisible({
    config,
    status,
    audioSetup: setup.audio,
    canDirectOutbound,
    canDirectInbound,
    canTranslateOutbound,
    canTranslateInbound,
  });

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      {error ? (
        <PipelineErrorBanner
          notice={error}
          onOpenAudioSettings={() => onOpenSettings("audio")}
          onUseSystemDefault={
            error.kind === "device-unavailable" && error.allowSystemDefault
              ? handleUseSystemDefault
              : undefined
          }
        />
      ) : null}

      <SetupBanner
        config={config}
        status={status}
        audioSetup={setup.audio}
        canDirectOutbound={canDirectOutbound}
        canDirectInbound={canDirectInbound}
        canTranslateOutbound={canTranslateOutbound}
        canTranslateInbound={canTranslateInbound}
        onOpenSettings={() => onOpenSettings("audio")}
      />

      <section className="relative flex min-h-0 flex-1 flex-col gap-0 p-0">
        <TranscriptPanel
          config={config}
          status={status}
          outboundColumn={outbound}
          inboundColumn={inbound}
          onSaveMode={onSaveConfig}
          onSetOutboundOutputMode={onSetOutboundOutputMode}
          onSetOutboundVoiceOutput={onSetOutboundVoiceOutput}
          onSetInboundOutputMode={onSetInboundOutputMode}
          onSetInboundVoiceOutput={onSetInboundVoiceOutput}
          onOutboundAudioPathChange={onOutboundAudioPathChange}
          onInboundAudioPathChange={onInboundAudioPathChange}
          onLongReconnectRestored={handleLongReconnectRestored}
          onLongAudioReconnectRestored={handleLongAudioReconnectRestored}
          onOpenSettings={() => onOpenSettings()}
          suppressSetupIdleChip={setupBannerVisible}
          onToast={showToast}
          micMuted={micMuted}
          onMicMuteToggle={onMicMuteToggle}
          speakerMuted={speakerMuted}
          onSpeakerMuteToggle={onSpeakerMuteToggle}
        />
      </section>
    </div>
  );
}
