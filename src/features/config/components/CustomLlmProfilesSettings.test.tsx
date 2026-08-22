import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";

import { TooltipProvider } from "@/shared/ui/tooltip";
import { baseConfig } from "@/test/fixtures/config";
import type { CustomLlmProfileView } from "@/shared/lib/types/pipeline";
import CustomLlmProfilesSettings from "./CustomLlmProfilesSettings";

const {
  upsertCustomLlmProfile,
  deleteCustomLlmProfile,
  testCustomLlmProfile,
} = vi.hoisted(() => ({
  upsertCustomLlmProfile: vi.fn(),
  deleteCustomLlmProfile: vi.fn(),
  testCustomLlmProfile: vi.fn(),
}));

vi.mock("@/shared/lib/api/configApi", () => ({
  upsertCustomLlmProfile,
  deleteCustomLlmProfile,
  testCustomLlmProfile,
}));

vi.mock("@/shared/components/SettingInfoHint", () => ({
  default: () => null,
}));

const ollama: CustomLlmProfileView = {
  id: "p1",
  label: "Ollama (local)",
  baseUrl: "http://localhost:11434",
  chatModel: "qwen3:8b",
  apiKeyConfigured: false,
};

const openrouter: CustomLlmProfileView = {
  id: "p2",
  label: "OpenRouter",
  baseUrl: "https://openrouter.ai/api/v1",
  chatModel: "anthropic/claude-sonnet-4",
  jsonMode: true,
  apiKeyConfigured: true,
};

function wrap(ui: ReactNode) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

function renderPanel(configOverrides: Record<string, unknown> = {}) {
  const onToast = vi.fn();
  render(
    wrap(
      <CustomLlmProfilesSettings
        config={{ ...baseConfig, ...configOverrides }}
        onToast={onToast}
      />,
    ),
  );
  return { onToast };
}

function openCreateForm() {
  fireEvent.click(screen.getByRole("button", { name: "Add custom provider" }));
}

function fillForm() {
  fireEvent.change(screen.getByLabelText("Name"), {
    target: { value: "Ollama (local)" },
  });
  fireEvent.change(screen.getByLabelText("Server URL"), {
    target: { value: "http://localhost:11434" },
  });
  fireEvent.change(screen.getByLabelText("Chat model"), {
    target: { value: "qwen3:8b" },
  });
}

describe("CustomLlmProfilesSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows an empty state when there are no profiles", () => {
    renderPanel({ customLlmProfiles: [] });
    expect(screen.getByText(/No custom providers yet/)).toBeTruthy();
  });

  it("lists profiles with url, model, and in-use badge", () => {
    renderPanel({
      customLlmProfiles: [ollama],
      summaryCustomProfileId: "p1",
    });
    expect(screen.getByText("Ollama (local)")).toBeTruthy();
    expect(screen.getByText("http://localhost:11434")).toBeTruthy();
    expect(screen.getByText(/qwen3:8b/)).toBeTruthy();
    expect(screen.getByText("In use")).toBeTruthy();
  });

  it("test connection success shows OK state", async () => {
    testCustomLlmProfile.mockResolvedValue(undefined);
    renderPanel({ customLlmProfiles: [] });
    openCreateForm();
    fillForm();
    fireEvent.click(screen.getByRole("button", { name: /Test connection/ }));

    await waitFor(() => {
      expect(screen.getByRole("status").textContent).toMatch(/Connection OK/);
    });
    expect(testCustomLlmProfile).toHaveBeenCalledWith({
      id: undefined,
      label: "Ollama (local)",
      baseUrl: "http://localhost:11434",
      chatModel: "qwen3:8b",
      apiKey: undefined,
    });
  });

  it("test connection failure shows the error and does not save", async () => {
    testCustomLlmProfile.mockRejectedValue(
      new Error("Chat test (streaming) failed: connection refused"),
    );
    renderPanel({ customLlmProfiles: [] });
    openCreateForm();
    fillForm();
    fireEvent.click(screen.getByRole("button", { name: /Test connection/ }));

    await waitFor(() => {
      expect(screen.getByRole("alert").textContent).toMatch(
        /connection refused/,
      );
    });
    expect(upsertCustomLlmProfile).not.toHaveBeenCalled();
  });

  it("save upserts the profile and closes the form", async () => {
    upsertCustomLlmProfile.mockResolvedValue(ollama);
    const { onToast } = renderPanel({ customLlmProfiles: [] });
    openCreateForm();
    fillForm();
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));

    await waitFor(() => {
      expect(upsertCustomLlmProfile).toHaveBeenCalledTimes(1);
    });
    expect(onToast).toHaveBeenCalledWith("success", "Provider profile added");
    await waitFor(() => {
      expect(screen.queryByLabelText("Server URL")).toBeNull();
    });
  });

  it("save surfaces the server-side test failure", async () => {
    upsertCustomLlmProfile.mockRejectedValue(
      new Error("Chat test (JSON mode) failed: 404"),
    );
    const { onToast } = renderPanel({ customLlmProfiles: [] });
    openCreateForm();
    fillForm();
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));

    await waitFor(() => {
      expect(onToast).toHaveBeenCalledWith(
        "error",
        expect.stringContaining("JSON mode"),
      );
    });
  });

  it("edit prefills the form and keeps the saved key when left empty", async () => {
    upsertCustomLlmProfile.mockResolvedValue(ollama);
    renderPanel({ customLlmProfiles: [ollama] });
    fireEvent.click(screen.getByRole("button", { name: "Edit Ollama (local)" }));

    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe(
      "Ollama (local)",
    );
    fireEvent.change(screen.getByLabelText("Chat model"), {
      target: { value: "qwen3:14b" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(upsertCustomLlmProfile).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "p1",
          chatModel: "qwen3:14b",
          apiKey: undefined,
        }),
      );
    });
  });

  it("edit with configured key shows keep hint and test omits apiKey", async () => {
    testCustomLlmProfile.mockResolvedValue(undefined);
    renderPanel({ customLlmProfiles: [openrouter] });
    fireEvent.click(screen.getByRole("button", { name: "Edit OpenRouter" }));

    expect(screen.getByTestId("custom-llm-key-saved").textContent).toMatch(
      /API key saved/,
    );
    fireEvent.click(screen.getByRole("button", { name: /Test connection/ }));

    await waitFor(() => {
      expect(testCustomLlmProfile).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "p2",
          apiKey: undefined,
        }),
      );
    });
  });

  it("save warns when JSON mode is unsupported", async () => {
    upsertCustomLlmProfile.mockResolvedValue({
      ...openrouter,
      jsonMode: false,
    });
    const { onToast } = renderPanel({ customLlmProfiles: [openrouter] });
    fireEvent.click(screen.getByRole("button", { name: "Edit OpenRouter" }));
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(onToast).toHaveBeenCalledWith(
        "success",
        expect.stringContaining("prompt-only JSON"),
      );
    });
  });

  it("delete confirms and warns when the profile is in use", async () => {
    deleteCustomLlmProfile.mockResolvedValue(undefined);
    const { onToast } = renderPanel({
      customLlmProfiles: [ollama],
      summaryCustomProfileId: "p1",
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Delete Ollama (local)" }),
    );

    expect(screen.getByText(/currently selected/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => {
      expect(deleteCustomLlmProfile).toHaveBeenCalledWith("p1");
    });
    expect(onToast).toHaveBeenCalledWith(
      "success",
      'Removed "Ollama (local)"',
    );
  });

  it("delete of an unused profile uses the neutral message", () => {
    renderPanel({ customLlmProfiles: [ollama], summaryCustomProfileId: null });
    fireEvent.click(
      screen.getByRole("button", { name: "Delete Ollama (local)" }),
    );
    expect(screen.getByText(/not affected/)).toBeTruthy();
    expect(screen.queryByText(/currently selected/)).toBeNull();
  });
});
