import type { ReactNode } from "react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { ToastProvider } from "@/shared/context/ToastProvider";
import { ThemeProvider } from "@/shared/context/ThemeProvider";
import { MeetingsProvider } from "@/features/meeting/library/context/MeetingsProvider";
import { PipelineRuntimeProvider } from "@/features/pipeline/context/PipelineRuntimeProvider";
import { TranscriptProvider } from "@/features/pipeline/context/transcript/TranscriptProvider";
import ErrorBoundary from "@/shared/components/ErrorBoundary";
import { ClockProvider } from "@/shared/context/ClockProvider";
import type { ThemePreference } from "@/shared/lib/theme";

export function AppProviders({
  children,
  themePreference,
  overlay = false,
}: {
  children: ReactNode;
  themePreference?: ThemePreference | null;
  /** Overlay window has its own runtime (useOverlayRuntime) — skip the main
  * pipeline/meeting/transcript providers so the webview doesn't double-
  * subscribe to `app-state` and transcript fan-out. */
  overlay?: boolean;
}) {
  const featureTree = overlay ? (
    <ErrorBoundary>{children}</ErrorBoundary>
  ) : (
    <PipelineRuntimeProvider>
      <MeetingsProvider>
        <TranscriptProvider>
          <ErrorBoundary>{children}</ErrorBoundary>
        </TranscriptProvider>
      </MeetingsProvider>
    </PipelineRuntimeProvider>
  );

  return (
    <ThemeProvider preference={themePreference}>
      <TooltipProvider>
        <ClockProvider>
          <ToastProvider>{featureTree}</ToastProvider>
        </ClockProvider>
      </TooltipProvider>
    </ThemeProvider>
  );
}
