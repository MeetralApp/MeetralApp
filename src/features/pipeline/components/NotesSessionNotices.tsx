import type { AppStatus } from "@/shared/lib/types/pipeline";
import LiveNotice from "@/shared/components/LiveNotice";
import { useAudioDeviceNotice } from "@/features/audio/hooks/useAudioDeviceNotice";
import { useBridgeConnectionNotice } from "@/features/audio/hooks/useBridgeConnectionNotice";
import {
  formatAudioReconnectAttemptSuffix,
  getAudioReconnectAttempt,
  MAX_AUDIO_RECONNECT_ATTEMPTS,
} from "@/features/audio/lib/audioConnection";
import {
  formatReconnectAttemptSuffix,
  getReconnectAttempt,
  MAX_RECONNECT_ATTEMPTS,
} from "@/features/audio/lib/bridgeConnection";
import { useSessionDriftHint } from "../hooks/useSessionDriftHint";
import { getBridgeState } from "@/features/audio/lib/bridgeConnection";
import { notesNoticeSides } from "../lib/notesNoticeSides";

interface Props {
  status: AppStatus | null;
  onLongReconnectRestored?: (columnTitle: string) => void;
  onLongAudioReconnectRestored?: (columnTitle: string) => void;
}

function attemptMeta(attempt: number | null, max: number): string | null {
  if (attempt == null) return null;
  if (attempt <= max) return `${attempt}/${max}`;
  return `${attempt}…`;
}

function maxAttempt(
  a: number | null,
  b: number | null,
): number | null {
  if (a == null) return b;
  if (b == null) return a;
  return Math.max(a, b);
}

/**
* Notes uses one shared header for both directions — collapse per-column
* reconnect / restore / drift rails so identical notices are not doubled.
*/
export default function NotesSessionNotices({
  status,
  onLongReconnectRestored,
  onLongAudioReconnectRestored,
}: Props) {
  const outboundAudio = useAudioDeviceNotice(status, "outbound", {
    onLongReconnectRestored: onLongAudioReconnectRestored,
  });
  const inboundAudio = useAudioDeviceNotice(status, "inbound", {
    onLongReconnectRestored: onLongAudioReconnectRestored,
  });
  const outboundBridge = useBridgeConnectionNotice(status, "outbound", {
    onLongReconnectRestored,
  });
  const inboundBridge = useBridgeConnectionNotice(status, "inbound", {
    onLongReconnectRestored,
  });
  const outboundDrift = useSessionDriftHint(status, "outbound");
  const inboundDrift = useSessionDriftHint(status, "inbound");

  const audioReconnecting =
    outboundAudio.mode === "reconnecting" ||
    inboundAudio.mode === "reconnecting";
  const audioRestored =
    !audioReconnecting &&
    (outboundAudio.mode === "reconnected" ||
      inboundAudio.mode === "reconnected");

  const bridgeReconnecting =
    outboundBridge.mode === "reconnecting" ||
    inboundBridge.mode === "reconnecting";
  const bridgeRestored =
    !bridgeReconnecting &&
    (outboundBridge.mode === "reconnected" ||
      inboundBridge.mode === "reconnected");

  const outboundBridgeBusy =
    getBridgeState(status, "outbound") === "reconnecting";
  const inboundBridgeBusy =
    getBridgeState(status, "inbound") === "reconnecting";
  const driftVisible =
    (outboundDrift.visible && !outboundBridgeBusy) ||
    (inboundDrift.visible && !inboundBridgeBusy);

  const audioReconnectSides = notesNoticeSides(
    outboundAudio.mode === "reconnecting",
    inboundAudio.mode === "reconnecting",
  );
  const audioRestoredSides = notesNoticeSides(
    outboundAudio.mode === "reconnected",
    inboundAudio.mode === "reconnected",
  );
  const bridgeReconnectSides = notesNoticeSides(
    outboundBridge.mode === "reconnecting",
    inboundBridge.mode === "reconnecting",
  );
  const bridgeRestoredSides = notesNoticeSides(
    outboundBridge.mode === "reconnected",
    inboundBridge.mode === "reconnected",
  );

  const audioAttempt = maxAttempt(
    outboundAudio.mode === "reconnecting"
      ? getAudioReconnectAttempt(status, "outbound")
      : null,
    inboundAudio.mode === "reconnecting"
      ? getAudioReconnectAttempt(status, "inbound")
      : null,
  );
  const bridgeAttempt = maxAttempt(
    outboundBridge.mode === "reconnecting"
      ? getReconnectAttempt(status, "outbound")
      : null,
    inboundBridge.mode === "reconnecting"
      ? getReconnectAttempt(status, "inbound")
      : null,
  );

  const dismissAudioRestored = () => {
    outboundAudio.dismiss();
    inboundAudio.dismiss();
  };
  const dismissBridgeRestored = () => {
    outboundBridge.dismiss();
    inboundBridge.dismiss();
  };
  const dismissDrift = () => {
    outboundDrift.dismiss();
    inboundDrift.dismiss();
  };

  return (
    <>
      {audioReconnecting ? (
        <LiveNotice.Rail
          icon="reconnecting"
          label="Reconnecting audio"
          meta={attemptMeta(audioAttempt, MAX_AUDIO_RECONNECT_ATTEMPTS)}
          detail={`${audioReconnectSides}: audio device disconnected — reconnecting…${formatAudioReconnectAttemptSuffix(audioAttempt)}`}
          ariaLabel={`${audioReconnectSides} audio device reconnecting`}
        />
      ) : null}
      {audioRestored ? (
        <LiveNotice.Rail
          icon="restored"
          label="Audio restored"
          detail={`${audioRestoredSides}: audio device reconnected.`}
          ariaLabel={`${audioRestoredSides} audio device reconnected`}
          onDismiss={dismissAudioRestored}
          dismissLabel="Dismiss audio device notice"
        />
      ) : null}

      {bridgeReconnecting ? (
        <LiveNotice.Rail
          icon="reconnecting"
          label="Reconnecting"
          meta={attemptMeta(bridgeAttempt, MAX_RECONNECT_ATTEMPTS)}
          detail={`${bridgeReconnectSides}: capture paused — reconnecting…${formatReconnectAttemptSuffix(bridgeAttempt)}`}
          ariaLabel={`${bridgeReconnectSides} reconnecting`}
        />
      ) : null}
      {bridgeRestored ? (
        <LiveNotice.Rail
          icon="restored"
          label="Connection restored"
          detail={`${bridgeRestoredSides}: connection restored. If captions drift, Stop then Start Notes.`}
          ariaLabel={`${bridgeRestoredSides} connection restored`}
          onDismiss={dismissBridgeRestored}
          dismissLabel="Dismiss connection notice"
        />
      ) : null}

      {driftVisible ? (
        <LiveNotice.Rail
          tone="neutral"
          icon="info"
          label="Long session"
          detail="Long session — if captions drift, Stop then Start Notes."
          ariaLabel="Notes session drift hint"
          onDismiss={dismissDrift}
          dismissLabel="Dismiss session drift hint"
        />
      ) : null}
    </>
  );
}
