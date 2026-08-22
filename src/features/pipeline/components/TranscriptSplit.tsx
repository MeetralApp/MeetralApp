import { Panel, PanelGroup } from "react-resizable-panels";

import { LiveTranscriptColumn } from "./LiveTranscriptColumn";
import NotesPipelineToolbar from "./NotesPipelineToolbar";
import NotesSessionNotices from "./NotesSessionNotices";
import AudioDeviceBanner from "@/features/audio/components/AudioDeviceBanner";
import ColumnPipelineToolbar from "./ColumnPipelineToolbar";
import ConnectionBanner from "@/features/audio/components/ConnectionBanner";
import SessionDriftHint from "./SessionDriftHint";
import TranscriptResizeHandle from "./TranscriptResizeHandle";
import {
  getOutboundToolbarModeOptions,
  getPipelineModeOptions,
} from "../lib/pipelineLabels";
import { isDirectionTranslating } from "../lib/pipelineStatus";
import { isNotesSession } from "../lib/sessionMode";
import type { AppStatus, AudioPathMode, ConfigView } from "@/shared/lib/types/pipeline";
import { cn } from "@/shared/lib/utils";
import type { ColumnUiState } from "../lib/columnUi";
import { pipelineToolbarClasses } from "../lib/pipelineColors";

interface Props {
  config: ConfigView;
  status: AppStatus | null;
  outboundColumn: ColumnUiState;
  inboundColumn: ColumnUiState;
  onOutboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onInboundAudioPathChange: (mode: AudioPathMode) => Promise<void>;
  onOutboundModeChange: (mode: string) => void | Promise<void>;
  onInboundModeChange: (mode: string) => void | Promise<void>;
  onLongReconnectRestored?: (columnTitle: string) => void;
  onLongAudioReconnectRestored?: (columnTitle: string) => void;
  onOpenSettings?: () => void;
  suppressSetupIdleChip?: boolean;
  micMuted?: boolean;
  onMicMuteToggle?: () => void;
  speakerMuted?: boolean;
  onSpeakerMuteToggle?: () => void;
}

export default function TranscriptSplit({
  config,
  status,
  outboundColumn,
  inboundColumn,
  onOutboundAudioPathChange,
  onInboundAudioPathChange,
  onOutboundModeChange,
  onInboundModeChange,
  onLongReconnectRestored,
  onLongAudioReconnectRestored,
  onOpenSettings,
  suppressSetupIdleChip = false,
  micMuted,
  onMicMuteToggle,
  speakerMuted,
  onSpeakerMuteToggle,
}: Props) {
  const notesMode = isNotesSession(config);

  if (notesMode) {
    return (
      <div
        className={cn(
          "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
          pipelineToolbarClasses.paneSurface,
        )}
      >
        <div
          className={cn(
            "sticky top-0 z-[4] shrink-0 has-[[data-state=open]]:z-[12]",
            pipelineToolbarClasses.paneToolbarRail,
          )}
        >
          <NotesSessionNotices
            status={status}
            onLongReconnectRestored={onLongReconnectRestored}
            onLongAudioReconnectRestored={onLongAudioReconnectRestored}
          />
          <NotesPipelineToolbar
            status={status}
            config={config}
            outboundColumn={outboundColumn}
            inboundColumn={inboundColumn}
            onOutboundAudioPathChange={onOutboundAudioPathChange}
            onInboundAudioPathChange={onInboundAudioPathChange}
            micMuted={micMuted}
            onMicMuteToggle={onMicMuteToggle}
            speakerMuted={speakerMuted}
            onSpeakerMuteToggle={onSpeakerMuteToggle}
            onLongReconnectRestored={onLongReconnectRestored}
            suppressSetupIdleChip={suppressSetupIdleChip}
          />
        </div>

        <PanelGroup
          direction="horizontal"
          autoSaveId="transcript-split-v1"
          className={cn(
            "min-h-0 flex-1 overflow-hidden rounded-none border-0 bg-transparent",
            "[&_[data-panel]]:flex [&_[data-panel]]:min-h-0 [&_[data-panel]]:min-w-0 [&_[data-panel]]:overflow-hidden",
            "[&_[data-panel]>_*]:min-h-0 [&_[data-panel]>_*]:min-w-0 [&_[data-panel]>_*]:flex-1",
          )}
        >
          <Panel id="you" minSize={28} defaultSize={50} order={1}>
            <LiveTranscriptColumn
              title="You"
              direction="outbound"
              placeholder="Your speech appears here as notes."
              transcriptLayout={config.transcriptLayout}
              transcriptVariant="notes"
              elevated={false}
              idleBadge={outboundColumn.idleBadge}
              onOpenSettings={onOpenSettings}
              followContent={isDirectionTranslating(status, "outbound")}
              banners={null}
              toolbar={null}
            />
          </Panel>
          <TranscriptResizeHandle inset />
          <Panel id="meeting" minSize={28} defaultSize={50} order={3}>
            <LiveTranscriptColumn
              title="Meeting"
              direction="inbound"
              placeholder="Meeting speech appears here as notes."
              transcriptLayout={config.transcriptLayout}
              transcriptVariant="notes"
              elevated={false}
              idleBadge={inboundColumn.idleBadge}
              onOpenSettings={onOpenSettings}
              followContent={isDirectionTranslating(status, "inbound")}
              banners={null}
              toolbar={null}
            />
          </Panel>
        </PanelGroup>
      </div>
    );
  }

  return (
    <PanelGroup
      direction="horizontal"
      autoSaveId="transcript-split-v1"
      className={cn(
        "min-h-0 flex-1 overflow-hidden rounded-none border-0 bg-transparent",
        "[&_[data-panel]]:flex [&_[data-panel]]:min-h-0 [&_[data-panel]]:min-w-0 [&_[data-panel]]:overflow-hidden",
        "[&_[data-panel]>_*]:min-h-0 [&_[data-panel]>_*]:min-w-0 [&_[data-panel]>_*]:flex-1",
      )}
    >
      <Panel id="you" minSize={28} defaultSize={50} order={1}>
        <LiveTranscriptColumn
          title="You"
          direction="outbound"
          placeholder="Your speech and translation appear here."
          transcriptLayout={config.transcriptLayout}
          transcriptVariant="bilingual"
          idleBadge={outboundColumn.idleBadge}
          onOpenSettings={onOpenSettings}
          followContent={isDirectionTranslating(status, "outbound")}
          banners={
            <>
              <AudioDeviceBanner
                direction="outbound"
                status={status}
                onLongReconnectRestored={onLongAudioReconnectRestored}
              />
              <ConnectionBanner
                direction="outbound"
                status={status}
                onLongReconnectRestored={onLongReconnectRestored}
              />
            </>
          }
          toolbar={
            <>
              <ColumnPipelineToolbar
                title="You"
                direction="outbound"
                status={status}
                config={config}
                columnUi={outboundColumn}
                modeOptions={getOutboundToolbarModeOptions(config)}
                onAudioPathChange={onOutboundAudioPathChange}
                onModeChange={onOutboundModeChange}
                micMuted={micMuted}
                onMicMuteToggle={onMicMuteToggle}
                onLongReconnectRestored={onLongReconnectRestored}
                suppressSetupIdleChip={suppressSetupIdleChip}
              />
              <SessionDriftHint direction="outbound" status={status} />
            </>
          }
        />
      </Panel>
      <TranscriptResizeHandle />
      <Panel id="meeting" minSize={28} defaultSize={50} order={3}>
        <LiveTranscriptColumn
          title="Meeting"
          direction="inbound"
          placeholder="Meeting speech and translation appear here."
          transcriptLayout={config.transcriptLayout}
          transcriptVariant="bilingual"
          idleBadge={inboundColumn.idleBadge}
          onOpenSettings={onOpenSettings}
          followContent={isDirectionTranslating(status, "inbound")}
          banners={
            <>
              <AudioDeviceBanner
                direction="inbound"
                status={status}
                onLongReconnectRestored={onLongAudioReconnectRestored}
              />
              <ConnectionBanner
                direction="inbound"
                status={status}
                onLongReconnectRestored={onLongReconnectRestored}
              />
            </>
          }
          toolbar={
            <>
              <ColumnPipelineToolbar
                title="Meeting"
                direction="inbound"
                status={status}
                config={config}
                columnUi={inboundColumn}
                modeOptions={getPipelineModeOptions("inbound", config)}
                onAudioPathChange={onInboundAudioPathChange}
                onModeChange={onInboundModeChange}
                speakerMuted={speakerMuted}
                onSpeakerMuteToggle={onSpeakerMuteToggle}
                onLongReconnectRestored={onLongReconnectRestored}
                suppressSetupIdleChip={suppressSetupIdleChip}
              />
              <SessionDriftHint direction="inbound" status={status} />
            </>
          }
        />
      </Panel>
    </PanelGroup>
  );
}
