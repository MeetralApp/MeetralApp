import { createContext } from "react";

import type { useTranslation } from "../hooks/useTranslation";

export type PipelineRuntimeValue = ReturnType<typeof useTranslation>;

export const PipelineRuntimeContext =
  createContext<PipelineRuntimeValue | null>(null);
