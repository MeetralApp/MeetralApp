import type { AppStatus } from "@/shared/lib/types/pipeline";
import LiveNotice from "@/shared/components/LiveNotice";
import { useAudioDeviceNotice } from "../hooks/useAudioDeviceNotice";
import {
  columnLabel,
  formatAudioReconnectAttemptSuffix,
  getAudioReconnectAttempt,
  MAX_AUDIO_RECONNECT_ATTEMPTS,
} from "../lib/audioConnection";
import type { PipelineDirection } from "@/features/pipeline/lib/sessionDrift";

interface Props {
  direction: PipelineDirection;
  status: AppStatus | null;
  onLongReconnectRestored?: (columnTitle: string) => void;
}

function attemptMeta(attempt: number | null): string | null {
  if (attempt == null) return null;
  if (attempt <= MAX_AUDIO_RECONNECT_ATTEMPTS) {
    return `${attempt}/${MAX_AUDIO_RECONNECT_ATTEMPTS}`;
  }
  return `${attempt}…`;
}

export default function AudioDeviceBanner({
  direction,
  status,
  onLongReconnectRestored,
}: Props) {
  const { mode, dismiss } = useAudioDeviceNotice(status, direction, {
    onLongReconnectRestored,
  });

  if (!mode) return null;

  const title = columnLabel(direction);
  const attempt = getAudioReconnectAttempt(status, direction);

  if (mode === "reconnecting") {
    const detail = `${title}: audio device disconnected — reconnecting…${formatAudioReconnectAttemptSuffix(attempt)}`;

    return (
      <LiveNotice.Rail
        icon="reconnecting"
        label="Reconnecting audio"
        meta={attemptMeta(attempt)}
        detail={detail}
        ariaLabel={`${title} audio device reconnecting`}
      />
    );
  }

  return (
    <LiveNotice.Rail
      icon="restored"
      label="Audio restored"
      detail={`${title}: audio device reconnected.`}
      ariaLabel={`${title} audio device reconnected`}
      onDismiss={dismiss}
      dismissLabel="Dismiss audio device notice"
    />
  );
}
