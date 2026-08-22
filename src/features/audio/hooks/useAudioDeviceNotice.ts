import { useCallback, useEffect, useRef, useState } from "react";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import { isInboundAudioActive, isOutboundAudioActive } from "@/features/pipeline/lib/pipelineStatus";
import {
  AUDIO_LONG_RECONNECT_TOAST_MS,
  AUDIO_RECONNECT_DEBOUNCE_MS,
  AUDIO_RECONNECTED_BANNER_MS,
  canShowAudioReconnectedBanner,
  columnLabel,
  getAudioState,
  writeAudioReconnectedCooldown,
  type AudioConnectionState,
} from "../lib/audioConnection";
import type { PipelineDirection } from "@/features/pipeline/lib/sessionDrift";

export type AudioDeviceNoticeMode = "reconnecting" | "reconnected" | null;

interface Options {
  onLongReconnectRestored?: (columnTitle: string) => void;
}

export function useAudioDeviceNotice(
  status: AppStatus | null,
  direction: PipelineDirection,
  options: Options = {},
): {
  mode: AudioDeviceNoticeMode;
  dismiss: () => void;
} {
  const { onLongReconnectRestored } = options;
  const [showReconnecting, setShowReconnecting] = useState(false);
  const [showReconnected, setShowReconnected] = useState(false);
  const prevAudioRef = useRef<AudioConnectionState>("ok");
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectedHideRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectingVisibleSinceRef = useRef<number | null>(null);

  const clearDebounce = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
  }, []);

  const clearReconnectedHide = useCallback(() => {
    if (reconnectedHideRef.current) {
      clearTimeout(reconnectedHideRef.current);
      reconnectedHideRef.current = null;
    }
  }, []);

  const dismiss = useCallback(() => {
    setShowReconnected(false);
    clearReconnectedHide();
  }, [clearReconnectedHide]);

  useEffect(() => {
    const audio = getAudioState(status, direction);
    const prev = prevAudioRef.current;
    prevAudioRef.current = audio;

    if (audio === "reconnecting" && prev !== "reconnecting") {
      clearDebounce();
      debounceRef.current = setTimeout(() => {
        setShowReconnecting(true);
        reconnectingVisibleSinceRef.current = Date.now();
      }, AUDIO_RECONNECT_DEBOUNCE_MS);
    }

    if (audio === "ok" && prev === "reconnecting") {
      clearDebounce();
      const visibleSince = reconnectingVisibleSinceRef.current;
      const wasReconnectingVisible = showReconnecting || visibleSince != null;
      const audioStillActive =
        direction === "outbound"
          ? isOutboundAudioActive(status)
          : isInboundAudioActive(status);
      setShowReconnecting(false);

      if (wasReconnectingVisible && audioStillActive) {
        const duration =
          visibleSince != null ? Date.now() - visibleSince : 0;
        if (duration >= AUDIO_LONG_RECONNECT_TOAST_MS) {
          onLongReconnectRestored?.(columnLabel(direction));
        }
        if (canShowAudioReconnectedBanner(direction)) {
          setShowReconnected(true);
          writeAudioReconnectedCooldown(direction);
          clearReconnectedHide();
          reconnectedHideRef.current = setTimeout(() => {
            setShowReconnected(false);
            reconnectedHideRef.current = null;
          }, AUDIO_RECONNECTED_BANNER_MS);
        }
      }
      reconnectingVisibleSinceRef.current = null;
    }

    if (audio === "ok" && prev !== "reconnecting") {
      clearDebounce();
      if (showReconnecting) {
        setShowReconnecting(false);
        reconnectingVisibleSinceRef.current = null;
      }
    }

    if (audio === "lost") {
      clearDebounce();
      setShowReconnecting(false);
      setShowReconnected(false);
      reconnectingVisibleSinceRef.current = null;
    }
  }, [
    status,
    direction,
    showReconnecting,
    clearDebounce,
    clearReconnectedHide,
    onLongReconnectRestored,
  ]);

  useEffect(
    () => () => {
      clearDebounce();
      clearReconnectedHide();
    },
    [clearDebounce, clearReconnectedHide],
  );

  const mode: AudioDeviceNoticeMode = showReconnecting
    ? "reconnecting"
    : showReconnected
      ? "reconnected"
      : null;

  return { mode, dismiss };
}
