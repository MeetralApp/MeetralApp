import type { OverlaySettings, TranscriptLayout } from "@/shared/lib/types/pipeline";
import { MOCK_OVERLAY_ROWS } from "../lib/mockTranscript";
import { createPreviewOverlayControls } from "../lib/previewControls";
import OverlayTranscriptView from "./OverlayTranscriptView";

type Props = {
  overlay: OverlaySettings;
  transcriptLayout: TranscriptLayout;
  notesMode?: boolean;
};

const PREVIEW_CONTROLS = createPreviewOverlayControls();

export default function OverlayPreviewCard({
  overlay,
  transcriptLayout,
  notesMode = false,
}: Props) {
  return (
    <div className="h-[200px] overflow-hidden rounded-md border border-border bg-card">
      <OverlayTranscriptView
        rows={MOCK_OVERLAY_ROWS}
        overlay={{ ...overlay, opacity: 1 }}
        transcriptLayout={transcriptLayout}
        notesMode={notesMode}
        title="Team standup"
        compact
        controls={{
          ...PREVIEW_CONTROLS,
          sessionMode: notesMode ? "notes" : "interpreter",
        }}
      />
    </div>
  );
}
