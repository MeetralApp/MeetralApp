import { useEffect, useState } from "react";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import { getBridgeState } from "@/features/audio/lib/bridgeConnection";
import {
  formatSessionElapsed,
  formatSessionElapsedTitle,
  getActiveSince,
  getPipelineState,
  getSessionElapsedMs,
  isSessionPastDriftThreshold,
  type PipelineDirection,
} from "../lib/sessionDrift";
import { useSharedClock } from "@/shared/context/useSharedClock";

export type SessionTimerVariant =
  | "starting"
  | "stopping"
  | "reconnecting"
  | "elapsed";

type Options = {
  /**
  * When true (overlay without ClockProvider), tick locally only while the
  * column session is starting/stopping/active/reconnecting — avoids a 1 Hz tree-wide clock.
  */
  tickWhenActive?: boolean;
};

export function usePipelineSessionElapsed(
  status: AppStatus | null,
  direction: PipelineDirection,
  options: Options = {},
): {
  display: string | null;
  variant: SessionTimerVariant | null;
  isLongSession: boolean;
  title: string | null;
} {
  const pipelineState = getPipelineState(status, direction);
  const bridgeState = getBridgeState(status, direction);
  const activeSince = getActiveSince(status, direction);
  const isActive = pipelineState === "active" && activeSince != null;
  const sharedNow = useSharedClock();
  const [localNow, setLocalNow] = useState(() => Date.now());

  const needsLocalTick =
    options.tickWhenActive === true &&
    (pipelineState === "starting" ||
      pipelineState === "stopping" ||
      pipelineState === "active" ||
      bridgeState === "reconnecting");

  useEffect(() => {
    if (!needsLocalTick) return;
    setLocalNow(Date.now());
    const id = window.setInterval(() => setLocalNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [needsLocalTick]);

  const now = needsLocalTick ? localNow : sharedNow;

  if (pipelineState === "starting") {
    return {
      display: "Starting…",
      variant: "starting",
      isLongSession: false,
      title: null,
    };
  }

  if (pipelineState === "stopping") {
    return {
      display: "Stopping…",
      variant: "stopping",
      isLongSession: false,
      title: null,
    };
  }

  if (pipelineState === "active" && bridgeState === "reconnecting") {
    return {
      display: "Reconnecting…",
      variant: "reconnecting",
      isLongSession: false,
      title: "Translation connection reconnecting",
    };
  }

  if (!isActive || activeSince == null) {
    return {
      display: null,
      variant: null,
      isLongSession: false,
      title: null,
    };
  }

  const elapsedMs = getSessionElapsedMs(activeSince, now);
  return {
    display: formatSessionElapsed(elapsedMs),
    variant: "elapsed",
    isLongSession: isSessionPastDriftThreshold(elapsedMs),
    title: formatSessionElapsedTitle(elapsedMs),
  };
}
