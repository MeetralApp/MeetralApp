import type { AppStatus } from "@/shared/lib/types/pipeline";
import LiveNotice from "@/shared/components/LiveNotice";
import { useBridgeConnectionNotice } from "../hooks/useBridgeConnectionNotice";
import {
  columnLabel,
  formatReconnectAttemptSuffix,
  getReconnectAttempt,
  MAX_RECONNECT_ATTEMPTS,
} from "../lib/bridgeConnection";
import type { PipelineDirection } from "@/features/pipeline/lib/sessionDrift";

interface Props {
  direction: PipelineDirection;
  status: AppStatus | null;
  onLongReconnectRestored?: (columnTitle: string) => void;
}

function attemptMeta(attempt: number | null): string | null {
  if (attempt == null) return null;
  if (attempt <= MAX_RECONNECT_ATTEMPTS) {
    return `${attempt}/${MAX_RECONNECT_ATTEMPTS}`;
  }
  return `${attempt}…`;
}

export default function ConnectionBanner({
  direction,
  status,
  onLongReconnectRestored,
}: Props) {
  const { mode, dismiss } = useBridgeConnectionNotice(status, direction, {
    onLongReconnectRestored,
  });

  if (!mode) return null;

  const title = columnLabel(direction);
  const attempt = getReconnectAttempt(status, direction);

  if (mode === "reconnecting") {
    const detail = `${title}: translation paused — playing original audio${formatReconnectAttemptSuffix(attempt)}`;

    return (
      <LiveNotice.Rail
        icon="reconnecting"
        label="Reconnecting translation"
        meta={attemptMeta(attempt)}
        detail={detail}
        ariaLabel={`${title} translation reconnecting`}
      />
    );
  }

  return (
    <LiveNotice.Rail
      icon="restored"
      label="Connection restored"
      detail={`${title}: connection restored. If voice or translation drifts, Stop then Start this pipeline.`}
      ariaLabel={`${title} connection restored`}
      onDismiss={dismiss}
      dismissLabel="Dismiss connection notice"
    />
  );
}
