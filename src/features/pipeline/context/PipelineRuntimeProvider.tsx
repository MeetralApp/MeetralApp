import type { ReactNode } from "react";

import { useTranslation } from "../hooks/useTranslation";
import { PipelineRuntimeContext } from "./PipelineRuntimeContext";

export function PipelineRuntimeProvider({ children }: { children: ReactNode }) {
  const value = useTranslation();
  return (
    <PipelineRuntimeContext.Provider value={value}>
      {children}
    </PipelineRuntimeContext.Provider>
  );
}
