import { useCallback, useEffect, useRef, useState } from "react";
import type { AppStatus } from "@/shared/lib/types/pipeline";
import {
  canShowReconnectedBanner,
  columnLabel,
  getBridgeState,
  LONG_RECONNECT_TOAST_MS,
  RECONNECT_DEBOUNCE_MS,
  RECONNECTED_BANNER_MS,
  writeReconnectedCooldown,
  type BridgeConnectionState,
} from "../lib/bridgeConnection";
import type { PipelineDirection } from "@/features/pipeline/lib/sessionDrift";

export type ConnectionNoticeMode = "reconnecting" | "reconnected" | null;

interface Options {
  onLongReconnectRestored?: (columnTitle: string) => void;
}

export function useBridgeConnectionNotice(
  status: AppStatus | null,
  direction: PipelineDirection,
  options: Options = {},
): {
  mode: ConnectionNoticeMode;
  dismiss: () => void;
} {
  const { onLongReconnectRestored } = options;
  const [showReconnecting, setShowReconnecting] = useState(false);
  const [showReconnected, setShowReconnected] = useState(false);
  const prevBridgeRef = useRef<BridgeConnectionState>("idle");
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
    const bridge = getBridgeState(status, direction);
    const prev = prevBridgeRef.current;
    prevBridgeRef.current = bridge;

    if (bridge === "reconnecting" && prev !== "reconnecting") {
      clearDebounce();
      debounceRef.current = setTimeout(() => {
        setShowReconnecting(true);
        reconnectingVisibleSinceRef.current = Date.now();
      }, RECONNECT_DEBOUNCE_MS);
    }

    if (bridge === "ready" && prev === "reconnecting") {
      clearDebounce();
      const visibleSince = reconnectingVisibleSinceRef.current;
      const wasReconnectingVisible = showReconnecting || visibleSince != null;
      setShowReconnecting(false);

      if (wasReconnectingVisible) {
        const duration =
          visibleSince != null ? Date.now() - visibleSince : 0;
        if (duration >= LONG_RECONNECT_TOAST_MS) {
          onLongReconnectRestored?.(columnLabel(direction));
        }
        if (canShowReconnectedBanner(direction)) {
          setShowReconnected(true);
          writeReconnectedCooldown(direction);
          clearReconnectedHide();
          reconnectedHideRef.current = setTimeout(() => {
            setShowReconnected(false);
            reconnectedHideRef.current = null;
          }, RECONNECTED_BANNER_MS);
        }
      }
      reconnectingVisibleSinceRef.current = null;
    }

    if (bridge === "ready" && prev !== "reconnecting") {
      clearDebounce();
      if (showReconnecting) {
        setShowReconnecting(false);
        reconnectingVisibleSinceRef.current = null;
      }
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

  const mode: ConnectionNoticeMode = showReconnecting
    ? "reconnecting"
    : showReconnected
      ? "reconnected"
      : null;

  return { mode, dismiss };
}
