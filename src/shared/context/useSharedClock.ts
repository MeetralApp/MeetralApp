import { useContext } from "react";

import { ClockContext } from "./clockContext";

export function useSharedClock(): number {
  const ctx = useContext(ClockContext);
  return ctx?.now ?? Date.now();
}
