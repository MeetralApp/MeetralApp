import { describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";

import { usePipelineRuntime } from "./usePipelineRuntime";

describe("usePipelineRuntime", () => {
  it("throws when used outside PipelineRuntimeProvider", () => {
    expect(() => renderHook(() => usePipelineRuntime())).toThrow(
      /PipelineRuntimeProvider/,
    );
  });
});
