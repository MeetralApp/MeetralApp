import { useContext } from "react";

import { MeetingsContext } from "./meetingsContext";

export function useMeetings() {
  const ctx = useContext(MeetingsContext);
  if (!ctx) {
    throw new Error("useMeetings must be used within MeetingsProvider");
  }
  return ctx;
}
