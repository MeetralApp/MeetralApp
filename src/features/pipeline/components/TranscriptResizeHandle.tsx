import { GripVertical } from "lucide-react";
import { PanelResizeHandle } from "react-resizable-panels";

import { pipelineToolbarClasses } from "@/features/pipeline/lib/pipelineColors";
import { cn } from "@/shared/lib/utils";

type Props = {
  /**
  * Inside a unified Notes shell: hairline split + grip (no gap between cards).
  * Default: mid-gutter grip only between elevated Interpreter panes.
  */
  inset?: boolean;
};

/** Mid-gutter grip — Interpreter uses transparent gutter; Notes inset adds a hairline. */
export default function TranscriptResizeHandle({ inset = false }: Props) {
  return (
    <PanelResizeHandle
      className={cn(
        pipelineToolbarClasses.resizeHandle,
        "group relative flex items-center justify-center outline-none",
        inset &&
          "before:pointer-events-none before:absolute before:inset-y-0 before:left-1/2 before:w-px before:-translate-x-1/2 before:bg-border/70",
      )}
      aria-label="Resize columns"
    >
      <span
        className={cn(
          "pointer-events-none z-[1] flex size-6 items-center justify-center rounded-md",
          "text-muted-foreground/55 transition-colors",
          "group-hover:bg-secondary group-hover:text-muted-foreground",
          "group-focus-visible:bg-secondary group-focus-visible:text-muted-foreground",
          "group-data-[resize-handle-active]:bg-secondary group-data-[resize-handle-active]:text-foreground",
        )}
      >
        <GripVertical className="size-3.5" aria-hidden strokeWidth={2} />
      </span>
    </PanelResizeHandle>
  );
}
