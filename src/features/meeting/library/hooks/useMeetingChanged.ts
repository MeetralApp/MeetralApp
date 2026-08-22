import { useEffect } from "react";

import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";
import type { MeetingChangedEvent } from "../lib/meetingTypes";

type MeetingChangedCallback = () => void;

const subscribers = new Set<MeetingChangedCallback>();
let refCount = 0;
let dispose: (() => void) | null = null;

function startMeetingChangedListener(): void {
  if (dispose) {
    return;
  }
  dispose = listenSafe<MeetingChangedEvent>(APP_EVENTS.meetingChanged, () => {
    for (const callback of subscribers) {
      callback();
    }
  });
}

function stopMeetingChangedListener(): void {
  dispose?.();
  dispose = null;
}

/** Single backend fan-out for `meeting-changed` — avoids duplicate Tauri listeners. */
export function useMeetingChanged(callback: MeetingChangedCallback): void {
  useEffect(() => {
    subscribers.add(callback);
    refCount += 1;
    startMeetingChangedListener();

    return () => {
      subscribers.delete(callback);
      refCount -= 1;
      if (refCount === 0) {
        stopMeetingChangedListener();
      }
    };
  }, [callback]);
}
