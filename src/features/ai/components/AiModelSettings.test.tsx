import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import { baseConfig } from "@/test/fixtures/config";
import AiModelSettings from "./AiModelSettings";

describe("AiModelSettings", () => {
  it("shows fixed OpenAI Notes STT model without writing liveModel", () => {
    const onSave = vi.fn();
    render(
      <AiModelSettings
        config={{ ...baseConfig, sessionMode: "notes", aiProvider: "openAi" }}
        models={[
          {
            id: "gpt-realtime-translate",
            name: "GPT Realtime Translate",
            languages: [],
          },
        ]}
        onSave={onSave}
        onToast={vi.fn()}
        notesSttModel={{
          id: "gpt-realtime-whisper",
          label: "GPT Realtime Whisper",
          description: "Notes STT",
        }}
      />,
    );

    expect(screen.getByLabelText("Notes STT model").textContent).toContain(
      "GPT Realtime Whisper",
    );
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("keeps live model select for Interpreter OpenAI", () => {
    render(
      <AiModelSettings
        config={{
          ...baseConfig,
          sessionMode: "interpreter",
          aiProvider: "openAi",
          liveModel: "gpt-realtime-translate",
        }}
        models={[
          {
            id: "gpt-realtime-translate",
            name: "GPT Realtime Translate",
            languages: [],
          },
        ]}
        onSave={vi.fn()}
        onToast={vi.fn()}
        notesSttModel={{
          id: "gpt-realtime-whisper",
          label: "GPT Realtime Whisper",
          description: "Notes STT",
        }}
      />,
    );

    expect(screen.getByLabelText("Live translate model")).toBeTruthy();
    expect(screen.getByText("GPT Realtime Translate")).toBeTruthy();
  });
});
