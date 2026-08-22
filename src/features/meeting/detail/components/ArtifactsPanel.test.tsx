import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import ArtifactsPanel from "./ArtifactsPanel";
import { ToastProvider } from "@/shared/context/ToastProvider";
import { TooltipProvider } from "@/shared/ui/tooltip";
import { DEFAULT_SUMMARY_TEMPLATE_ID } from "../lib/summaryConstants";
import type {
  MeetingArtifact,
  MeetingEntity,
  MeetingRecord,
  MeetingSummary,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";

const meeting: MeetingRecord = {
  id: "m1",
  title: "Weekly sync",
  status: "ended",
  folderId: null,
  myLanguage: "en",
  meetingLanguage: "vi",
  startedAtMs: 1_700_000_000_000,
  endedAtMs: 1_700_000_360_000,
  segmentCountOutbound: 1,
  segmentCountInbound: 1,
};

const summary: MeetingSummary = {
  meetingId: "m1",
  templateId: DEFAULT_SUMMARY_TEMPLATE_ID,
  summaryLanguage: "vi",
  generatedJson: "{}",
  generatedText: "",
  generatedAtMs: 1_700_000_400_000,
};

const artifacts: MeetingArtifact[] = [
  {
    id: "m1:decision:1",
    kind: "decision",
    text: "Use Redis Cluster",
    status: "proposed",
    origin: "extracted",
    citations: [
      { segmentId: "seg-1", direction: "outbound", startedAtMs: 65_000 },
    ],
  },
  {
    id: "m1:action_item:1",
    kind: "action_item",
    text: "Prepare migration plan",
    owner: "Alice",
    due: "Friday",
    status: "proposed",
    origin: "extracted",
    citations: [],
  },
  {
    id: "m1:open_question:1",
    kind: "open_question",
    text: "Who owns rollback?",
    status: "proposed",
    origin: "extracted",
    citations: [],
  },
];

const entities: MeetingEntity[] = [
  { id: "m1:entity:AUTH-123", name: "AUTH-123", kind: "ticket", origin: "extracted" },
];

const segments: TranscriptSegment[] = [
  {
    id: "seg-1",
    meetingId: "m1",
    direction: "outbound",
    sequence: 1,
    sourceText: "Let's use Redis Cluster",
    translatedText: "Dùng Redis Cluster nhé",
    startedAtMs: 65_000,
    endedAtMs: 67_000,
    connectionGap: false,
  },
];

function mockInvoke(
  nextArtifacts: MeetingArtifact[] = artifacts,
  nextEntities: MeetingEntity[] = entities,
) {
  vi.mocked(invoke).mockImplementation(async (cmd, args) => {
    if (cmd === "list_meeting_artifacts") return nextArtifacts;
    if (cmd === "list_meeting_entities") return nextEntities;
    if (cmd === "set_artifact_status") {
      const { id, status } = (args as { request: { id: string; status: string } })
        .request;
      const found = nextArtifacts.find((artifact) => artifact.id === id);
      return { ...found, status };
    }
    if (cmd === "update_meeting_artifact") {
      const { id, text, owner, due } = (
        args as {
          request: {
            id: string;
            text: string;
            owner: string | null;
            due: string | null;
          };
        }
      ).request;
      const found = nextArtifacts.find((artifact) => artifact.id === id);
      return {
        ...found,
        id: id.includes(":u") ? id : `${id.replace(/:\d+$/, "")}:uabcd1234`,
        text,
        owner: owner ?? undefined,
        due: due ?? undefined,
        origin: "edited",
        citations: found?.citations ?? [],
        status: found?.status ?? "proposed",
        kind: found?.kind ?? "decision",
      };
    }
    if (cmd === "create_meeting_artifact") {
      const { meetingId, kind, text, owner, due } = (
        args as {
          request: {
            meetingId: string;
            kind: string;
            text: string;
            owner: string | null;
            due: string | null;
          };
        }
      ).request;
      return {
        id: `${meetingId}:${kind}:u11111111`,
        kind,
        text,
        owner: owner ?? undefined,
        due: due ?? undefined,
        status: "proposed",
        origin: "manual",
        citations: [],
      };
    }
    if (cmd === "delete_meeting_artifact") {
      return true;
    }
    if (cmd === "update_meeting_entity") {
      const { id, name, kind } = (
        args as { request: { id: string; name: string; kind: string } }
      ).request;
      return { id, name, kind, origin: "edited" };
    }
    if (cmd === "create_meeting_entity") {
      const { meetingId, name, kind } = (
        args as { request: { meetingId: string; name: string; kind: string } }
      ).request;
      if (name === "AUTH-123") {
        throw new Error("entity already exists: AUTH-123");
      }
      return {
        id: `${meetingId}:entity:${name}`,
        name,
        kind,
        origin: "manual",
      };
    }
    if (cmd === "delete_meeting_entity") {
      return true;
    }
    return undefined;
  });
}

function renderPanel(
  overrides: Partial<Parameters<typeof ArtifactsPanel>[0]> = {},
) {
  const props = {
    meeting,
    summary,
    onCitationClick: vi.fn(),
    segments,
    ...overrides,
  };
  render(
    <ToastProvider>
      <TooltipProvider>
        <ArtifactsPanel {...props} />
      </TooltipProvider>
    </ToastProvider>,
  );
  return props;
}

describe("ArtifactsPanel", () => {
  beforeEach(() => {
    mockInvoke();
  });

  async function openActionsMenu(triggerName: string | RegExp) {
    const trigger = await screen.findByLabelText(triggerName);
    // Radix DropdownMenuTrigger opens on pointerDown (not click).
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    return within(await screen.findByRole("menu"));
  }

  async function openArtifactActions(artifactText: string) {
    const text = await screen.findByText(artifactText);
    const row = text.closest("li");
    expect(row).toBeTruthy();
    const trigger = within(row as HTMLElement).getByLabelText(
      "Artifact actions",
    );
    fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
    return within(await screen.findByRole("menu"));
  }

  it("renders three sections with items and entity chips", async () => {
    renderPanel();
    expect(await screen.findByText("Decisions")).toBeTruthy();
    expect(screen.getByText("Action items")).toBeTruthy();
    expect(screen.getByText("Open questions")).toBeTruthy();
    expect(screen.getByText("Use Redis Cluster")).toBeTruthy();
    expect(screen.getByText("Owner: Alice")).toBeTruthy();
    expect(screen.getByText("Due: Friday")).toBeTruthy();
    expect(screen.getByText("AUTH-123")).toBeTruthy();
  });

  it("confirm flow flips the checklist toggle without a text badge", async () => {
    renderPanel();
    const confirmButtons = await screen.findAllByLabelText("Confirm artifact");
    fireEvent.click(confirmButtons[0]);
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("set_artifact_status", {
      request: { id: "m1:decision:1", status: "confirmed" },
    });

    // State lives on the toggle (aria-pressed) — never a "Confirmed" text badge.
    const undoToggle = await screen.findByLabelText("Undo confirm artifact");
    expect(undoToggle.getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByText("Confirmed")).toBeNull();
  });

  it("undo confirm returns the artifact to proposed", async () => {
    mockInvoke(
      artifacts.map((artifact) =>
        artifact.id === "m1:decision:1"
          ? { ...artifact, status: "confirmed" as const }
          : artifact,
      ),
    );
    renderPanel();

    const undoToggle = await screen.findByLabelText("Undo confirm artifact");
    fireEvent.click(undoToggle);
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("set_artifact_status", {
      request: { id: "m1:decision:1", status: "proposed" },
    });
    await waitFor(() => {
      expect(
        screen.getAllByLabelText("Confirm artifact")[0]?.getAttribute(
          "aria-pressed",
        ),
      ).toBe("false");
    });
  });

  it("dismiss flow hides the artifact", async () => {
    renderPanel();
    const menu = await openArtifactActions("Use Redis Cluster");
    fireEvent.click(menu.getByRole("menuitem", { name: "Dismiss" }));
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("set_artifact_status", {
      request: { id: "m1:decision:1", status: "dismissed" },
    });
    await waitFor(() => {
      expect(screen.queryByText("Use Redis Cluster")).toBeNull();
    });
  });

  it("citation chip jumps to the segment", async () => {
    const props = renderPanel();
    const chip = await screen.findByLabelText(/Jump to 1:05/);
    fireEvent.click(chip);
    expect(props.onCitationClick).toHaveBeenCalledWith(
      expect.objectContaining({ segmentId: "seg-1" }),
    );
  });

  it("empty state points to the Generate dock flow without a duplicate CTA", async () => {
    mockInvoke([], []);
    renderPanel();
    expect(
      await screen.findByText(/open Summary options below to generate/),
    ).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Extract artifacts" }),
    ).toBeNull();
    expect(screen.getByRole("button", { name: "Add decision" })).toBeTruthy();
  });

  it("live meeting shows guidance without extract button", async () => {
    mockInvoke([], []);
    renderPanel({ meeting: { ...meeting, status: "live" } });
    expect(
      await screen.findByText(/Artifacts appear after the meeting ends/),
    ).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Extract artifacts" }),
    ).toBeNull();
  });

  it("no transcript shows guidance without extract button", async () => {
    mockInvoke([], []);
    renderPanel({ segments: [] });
    expect(
      await screen.findByText(/No transcript to analyze/),
    ).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Extract artifacts" }),
    ).toBeNull();
  });

  it("edit flow saves and replaces the row from the returned view", async () => {
    renderPanel();
    const menu = await openArtifactActions("Use Redis Cluster");
    fireEvent.click(menu.getByRole("menuitem", { name: "Edit" }));
    const textArea = screen.getByLabelText("Artifact text");
    fireEvent.change(textArea, {
      target: { value: "Use Redis Cluster (edited)" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith("update_meeting_artifact", {
        request: {
          id: "m1:decision:1",
          text: "Use Redis Cluster (edited)",
          owner: null,
          due: null,
        },
      });
    });
    expect(
      await screen.findByText("Use Redis Cluster (edited)"),
    ).toBeTruthy();
  });

  it("add decision creates a manual row", async () => {
    renderPanel();
    await screen.findByText("Decisions");
    fireEvent.click(screen.getByRole("button", { name: "Add decision" }));
    fireEvent.change(screen.getByLabelText("Artifact text"), {
      target: { value: "Ship auth service" },
    });
    // Form save label is "Add"; entity chip uses aria-label "Add entity".
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith("create_meeting_artifact", {
        request: {
          meetingId: "m1",
          kind: "decision",
          text: "Ship auth service",
          owner: null,
          due: null,
        },
      });
    });
    expect(await screen.findByText("Ship auth service")).toBeTruthy();
  });

  it("two-click delete removes the artifact", async () => {
    renderPanel();
    const menu = await openArtifactActions("Use Redis Cluster");
    fireEvent.click(menu.getByRole("menuitem", { name: "Delete artifact" }));
    expect(
      menu.getByRole("menuitem", { name: "Confirm delete artifact" }),
    ).toBeTruthy();
    fireEvent.click(
      menu.getByRole("menuitem", { name: "Confirm delete artifact" }),
    );
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith("delete_meeting_artifact", {
        request: { id: "m1:decision:1" },
      });
    });
    await waitFor(() => {
      expect(screen.queryByText("Use Redis Cluster")).toBeNull();
    });
  });

  it("idle artifact rows do not reserve padding for the actions menu", async () => {
    renderPanel();
    const text = await screen.findByText("Use Redis Cluster");
    const row = text.closest("li");
    expect(row).toBeTruthy();
    expect(row?.className.includes("pr-16")).toBe(false);
    expect(
      within(row as HTMLElement).getByLabelText("Artifact actions"),
    ).toBeTruthy();
  });

  it("entity rename opens from the chip actions menu", async () => {
    renderPanel();
    await screen.findByText("AUTH-123");
    const menu = await openActionsMenu("Entity actions for AUTH-123");
    fireEvent.click(menu.getByRole("menuitem", { name: "Rename" }));
    expect(screen.getByLabelText("Entity name")).toBeTruthy();
    expect(screen.getByLabelText("Entity kind")).toBeTruthy();
  });

  it("entity two-click delete removes the chip", async () => {
    renderPanel();
    await screen.findByText("AUTH-123");
    const menu = await openActionsMenu("Entity actions for AUTH-123");
    fireEvent.click(
      menu.getByRole("menuitem", { name: "Delete entity AUTH-123" }),
    );
    fireEvent.click(
      menu.getByRole("menuitem", { name: "Confirm delete entity AUTH-123" }),
    );
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith("delete_meeting_entity", {
        request: { id: "m1:entity:AUTH-123" },
      });
    });
    await waitFor(() => {
      expect(screen.queryByText("AUTH-123")).toBeNull();
    });
  });

  it("entity add conflict surfaces a toast", async () => {
    renderPanel();
    await screen.findByText("AUTH-123");
    fireEvent.click(screen.getByRole("button", { name: "Add entity" }));
    fireEvent.change(screen.getByLabelText("Entity name"), {
      target: { value: "AUTH-123" },
    });
    fireEvent.change(screen.getByLabelText("Entity kind"), {
      target: { value: "ticket" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByText(/Entity already exists/)).toBeTruthy();
  });
});
