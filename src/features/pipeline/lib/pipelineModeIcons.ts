import type { ComponentType } from "react";
import {
  AudioWaveform,
  Captions,
  Languages,
  NotebookPen,
  Sparkles,
  UserRound,
  Zap,
} from "lucide-react";

export type LucideIcon = ComponentType<{
  className?: string;
  "aria-hidden"?: boolean | "true" | "false";
}>;

/** Path icons — Direct / Translate / Notes. */
export const DirectPathIcon = Zap;
export const TranslatePathIcon = Languages;
export const NotesPathIcon = NotebookPen;

/** Output mode icons for Translate dropdown / active chip. */
export function pipelineOutputModeIcon(value: string): LucideIcon {
  switch (value) {
    case "translatedClone":
      return UserRound;
    case "originalAudio":
      return AudioWaveform;
    case "textOnly":
      return Captions;
    case "translated":
    default:
      return Sparkles;
  }
}
