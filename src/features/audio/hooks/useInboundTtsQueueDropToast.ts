import { useEffect } from "react";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";

import { useToast } from "@/shared/context/useToast";

const DROP_TOAST =
  "Meeting translation backlog is full — newest speech was skipped so playback stays smooth.";

/** Toast when underlay TTS queue drops newest chunks (backend debounces emits). */
export function useInboundTtsQueueDropToast() {
  const { showToast } = useToast();

  useEffect(
    () =>
      listenSafe(APP_EVENTS.inboundTtsQueueDrop, () => {
        showToast("warning", DROP_TOAST);
      }),
    [showToast],
  );
}
