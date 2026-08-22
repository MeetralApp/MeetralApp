import { useEffect, useState } from "react";
import { listenSafe } from "@/shared/lib/listenSafe";
import { APP_EVENTS } from "@/shared/lib/events";
import { useToast } from "@/shared/context/useToast";
import type { VoiceTtsStatusPayload } from "../lib/voiceTypes";

export function useVoiceTtsStatus(enabled: boolean) {
  const { showToast } = useToast();
  const [degradedMessage, setDegradedMessage] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    if (!enabled) {
      setDegradedMessage(null);
      setReady(false);
      return;
    }

    return listenSafe<VoiceTtsStatusPayload>(APP_EVENTS.voiceTtsStatus, (event) => {
      if (event.payload.kind === "ready") {
        setReady(true);
        setDegradedMessage(null);
      } else {
        setReady(false);
        setDegradedMessage(event.payload.message);
        showToast("error", event.payload.message);
      }
    });
  }, [enabled, showToast]);

  return { ready, degradedMessage };
}
