import type { PipelineModeOption } from "@/features/pipeline/lib/pipelineLabels";
import type { OverlayColumnRuntime } from "./overlayRuntimeTypes";
import type { OverlayControlsProps } from "../components/OverlayTranscriptView";

function previewColumn(
  pathMode: OverlayColumnRuntime["pathMode"],
  idleLabel: string,
  idleKind: string = "ready",
): OverlayColumnRuntime {
  return {
    column: null,
    pathMode,
    muteEnabled: false,
    canDirect: false,
    canTranslate: false,
    audioFault: false,
    awaitingDirectStandby: false,
    translateDisabled: true,
    directDisabled: true,
    idleKind,
    idleLabel,
    idleTitle: "Preview — controls disabled",
  };
}

const OUTBOUND_OPTIONS: PipelineModeOption[] = [
  { value: "translated", label: "Translated voice", shortLabel: "Translated" },
  { value: "translatedClone", label: "My cloned voice", shortLabel: "Clone" },
  { value: "originalAudio", label: "My voice (raw)", shortLabel: "Raw" },
  { value: "textOnly", label: "Captions only", shortLabel: "Captions" },
];

const INBOUND_OPTIONS: PipelineModeOption[] = [
  { value: "translated", label: "Translated audio", shortLabel: "Translated" },
  { value: "originalAudio", label: "Meeting (raw)", shortLabel: "Raw" },
  { value: "textOnly", label: "Captions only", shortLabel: "Captions" },
];

/** Read-only control chrome matching the live overlay layout. */
export function createPreviewOverlayControls(): OverlayControlsProps {
  const noop = () => {};
  return {
    status: null,
    outbound: previewColumn("translate", "12:04"),
    inbound: previewColumn("direct", "Ready"),
    micMuted: false,
    speakerMuted: false,
    outboundToolbarMode: "translatedClone",
    inboundMode: "translated",
    outboundModeOptions: OUTBOUND_OPTIONS,
    inboundModeOptions: INBOUND_OPTIONS,
    cloneActive: true,
    onMicToggle: noop,
    onSpeakerToggle: noop,
    onOutboundPath: noop,
    onInboundPath: noop,
    onOutputModeChange: noop,
    onCycleOpacity: noop,
  };
}
