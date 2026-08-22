import type { ColumnUiState } from "@/features/pipeline/lib/columnUi";
import type {
  InboundToolbarMode,
  OutboundToolbarMode,
  PipelineModeOption,
} from "@/features/pipeline/lib/pipelineLabels";
import type {
  AppStatus,
  AudioPathMode,
  ConfigView,
  OverlaySettings,
  TranscriptLayout,
} from "@/shared/lib/types/pipeline";

export type OverlayColumnRuntime = {
  column: ColumnUiState | null;
  pathMode: AudioPathMode;
  muteEnabled: boolean;
  canDirect: boolean;
  canTranslate: boolean;
  audioFault: boolean;
  awaitingDirectStandby: boolean;
  translateDisabled: boolean;
  directDisabled: boolean;
  idleKind: string | null;
  idleLabel: string | null;
  idleTitle: string | null;
};

export type OverlayRuntime = {
  status: AppStatus | null;
  config: ConfigView | null;
  overlay: OverlaySettings;
  transcriptLayout: TranscriptLayout;
  outbound: OverlayColumnRuntime;
  inbound: OverlayColumnRuntime;
  micMuted: boolean;
  speakerMuted: boolean;
  /** Outbound toolbar value including translatedClone. */
  outboundToolbarMode: OutboundToolbarMode;
  /** Inbound toolbar value including translatedClone. */
  inboundMode: InboundToolbarMode;
  outboundModeOptions: PipelineModeOption[];
  inboundModeOptions: PipelineModeOption[];
  cloneActive: boolean;
  toggleMicMuted: () => Promise<void>;
  toggleSpeakerMuted: () => Promise<void>;
  setOutboundPath: (mode: AudioPathMode) => Promise<void>;
  setInboundPath: (mode: AudioPathMode) => Promise<void>;
  setOutputMode: (
    direction: "outbound" | "inbound",
    toolbarValue: string,
  ) => Promise<void>;
};
