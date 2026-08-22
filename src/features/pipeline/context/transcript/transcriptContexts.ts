import { createContext } from "react";

import type { LiveTranscriptAction } from "./liveTranscriptReducer";
import type {
  CommittedSlice,
  DirectionTranscriptSlice,
  InterimSlice,
} from "./liveTranscriptReducer";

export const OutboundTranscriptContext = createContext<DirectionTranscriptSlice | null>(null);
export const InboundTranscriptContext = createContext<DirectionTranscriptSlice | null>(null);

/** @deprecated Internal — use per-direction hooks */
export const InterimContext = createContext<InterimSlice | null>(null);
/** @deprecated Internal — use per-direction hooks */
export const CommittedContext = createContext<CommittedSlice | null>(null);

export type DispatchContextValue = {
  dispatch: React.Dispatch<LiveTranscriptAction>;
  activeMeetingId: string | null;
  clearTranscripts: () => void;
};

export const DispatchContext = createContext<DispatchContextValue | null>(null);
