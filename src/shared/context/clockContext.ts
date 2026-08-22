import { createContext } from "react";

export type ClockContextValue = {
  now: number;
};

export const ClockContext = createContext<ClockContextValue | null>(null);
