import { useCallback, useEffect, useState } from "react";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import {
  getActiveSince,
  readDismissed,
  shouldShowSessionDriftHint,
  syncDismissForSession,
  writeDismissed,
  type PipelineDirection,
} from "../lib/sessionDrift";
import { useSharedClock } from "@/shared/context/useSharedClock";

export function useSessionDriftHint(
  status: AppStatus | null,
  direction: PipelineDirection,
): { visible: boolean; dismiss: () => void } {
  const activeSince = getActiveSince(status, direction);
  const now = useSharedClock();
  const [dismissed, setDismissed] = useState(() => readDismissed(direction));

  useEffect(() => {
    syncDismissForSession(direction, activeSince);
    setDismissed(readDismissed(direction));
  }, [direction, activeSince]);

  const dismiss = useCallback(() => {
    writeDismissed(direction, true);
    setDismissed(true);
  }, [direction]);

  const visible = shouldShowSessionDriftHint({
    status,
    direction,
    now,
    dismissed,
  });

  return { visible, dismiss };
}