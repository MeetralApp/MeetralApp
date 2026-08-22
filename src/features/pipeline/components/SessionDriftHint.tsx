import type { AppStatus } from "@/shared/lib/types/pipeline";
import LiveNotice from "@/shared/components/LiveNotice";
import { useSessionDriftHint } from "../hooks/useSessionDriftHint";
import { getBridgeState } from "@/features/audio/lib/bridgeConnection";
import type { PipelineDirection } from "../lib/sessionDrift";

interface Props {
  direction: PipelineDirection;
  status: AppStatus | null;
}

export default function SessionDriftHint({ direction, status }: Props) {
  const { visible, dismiss } = useSessionDriftHint(status, direction);
  const bridgeState = getBridgeState(status, direction);

  if (!visible || bridgeState === "reconnecting") return null;

  const label =
    direction === "outbound"
      ? "You pipeline session drift hint"
      : "Meeting pipeline session drift hint";

  const detail =
    "Long session — if translation or voice drifts, Stop then Start this pipeline.";

  return (
    <LiveNotice.Rail
      tone="neutral"
      icon="info"
      label="Long session"
      detail={detail}
      ariaLabel={label}
      onDismiss={dismiss}
      dismissLabel="Dismiss session drift hint"
    />
  );
}
