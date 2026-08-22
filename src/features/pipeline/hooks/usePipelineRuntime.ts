import { useContext } from "react";

import { PipelineRuntimeContext } from "../context/PipelineRuntimeContext";

export function usePipelineRuntime() {
  const ctx = useContext(PipelineRuntimeContext);
  if (!ctx) {
    throw new Error(
      "usePipelineRuntime must be used within PipelineRuntimeProvider",
    );
  }
  return ctx;
}
