import { useCallback, useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  ChevronDown,
  Circle,
  HelpCircle,
  ListChecks,
  MoreHorizontal,
  Pencil,
  Plus,
  Sparkles,
  Tag,
  Trash2,
  X,
  type LucideIcon,
} from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { useToast } from "@/shared/context/useToast";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { Input } from "@/shared/ui/input";
import { Textarea } from "@/shared/ui/textarea";
import { cn } from "@/shared/lib/utils";
import { CitationChip } from "./CitationChip";
import {
  describeCitationFromSegments,
  type DescribeCitation,
} from "../lib/citationDescription";
import {
  createMeetingArtifact,
  createMeetingEntity,
  deleteMeetingArtifact,
  deleteMeetingEntity,
  listMeetingArtifacts,
  listMeetingEntities,
  setArtifactStatus,
  updateMeetingArtifact,
  updateMeetingEntity,
} from "@/features/meeting/library/lib/meetingApi";
import type {
  ArtifactKind,
  ArtifactStatus,
  SegmentCitation,
  MeetingArtifact,
  MeetingEntity,
  MeetingRecord,
  MeetingSummary,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";

interface Props {
  meeting: MeetingRecord;
  summary: MeetingSummary | null;
  onCitationClick: (citation: SegmentCitation) => void;
  /** Detail transcript — resolves citation snippet tooltips. */
  segments?: TranscriptSegment[];
}

const SECTIONS = [
  { kind: "decision" as const, title: "Decisions", addLabel: "Add decision", Icon: CheckCircle2 },
  { kind: "action_item" as const, title: "Action items", addLabel: "Add action item", Icon: ListChecks },
  { kind: "open_question" as const, title: "Open questions", addLabel: "Add open question", Icon: HelpCircle },
] as const;

type ArtifactDraft = {
  text: string;
  owner: string;
  due: string;
};

type EntityDraft = {
  name: string;
  kind: string;
};

function emptyArtifactDraft(): ArtifactDraft {
  return { text: "", owner: "", due: "" };
}

function emptyEntityDraft(): EntityDraft {
  return { name: "", kind: "" };
}

export default function ArtifactsPanel({
  meeting,
  summary,
  onCitationClick,
  segments,
}: Props) {
  const { showToast } = useToast();
  const [artifacts, setArtifacts] = useState<MeetingArtifact[]>([]);
  const [entities, setEntities] = useState<MeetingEntity[]>([]);
  const [collapsed, setCollapsed] = useState(false);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState<ArtifactDraft>(emptyArtifactDraft);
  const [addingKind, setAddingKind] = useState<ArtifactKind | null>(null);
  const [addDraft, setAddDraft] = useState<ArtifactDraft>(emptyArtifactDraft);
  /** Shared two-click arm for artifact + entity deletes (id namespaces don't collide). */
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [editingEntityId, setEditingEntityId] = useState<string | null>(null);
  const [entityEditDraft, setEntityEditDraft] = useState<EntityDraft>(
    emptyEntityDraft,
  );
  const [addingEntity, setAddingEntity] = useState(false);
  const [entityAddDraft, setEntityAddDraft] = useState<EntityDraft>(
    emptyEntityDraft,
  );

  // Citation snippets: resolve from the detail transcript already loaded by
  // the layout — no extra IPC.
  const describeCitation = useMemo<DescribeCitation | undefined>(() => {
    if (!segments?.length) return undefined;
    return describeCitationFromSegments(
      new Map(segments.map((segment) => [segment.id, segment])),
    );
  }, [segments]);

  const summaryGeneratedAtMs = summary?.generatedAtMs ?? null;

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      listMeetingArtifacts(meeting.id),
      listMeetingEntities(meeting.id),
    ])
      .then(([nextArtifacts, nextEntities]) => {
        if (cancelled) return;
        setArtifacts(nextArtifacts ?? []);
        setEntities(nextEntities ?? []);
      })
      .catch(() => {
      // Keep stale/empty — feature may be disabled mid-session.
      });
    return () => {
      cancelled = true;
    };
  }, [meeting.id, summaryGeneratedAtMs]);

  const visible = useMemo(
    () => artifacts.filter((artifact) => artifact.status !== "dismissed"),
    [artifacts],
  );

  const resetCurationUi = useCallback(() => {
    setEditingId(null);
    setEditDraft(emptyArtifactDraft());
    setAddingKind(null);
    setAddDraft(emptyArtifactDraft());
    setConfirmDeleteId(null);
    setEditingEntityId(null);
    setEntityEditDraft(emptyEntityDraft());
    setAddingEntity(false);
    setEntityAddDraft(emptyEntityDraft());
  }, []);

  /** First click arms Confirm; second click (same id) returns true to run delete. */
  const armOrConfirmDelete = useCallback(
    (id: string): boolean => {
      if (confirmDeleteId !== id) {
        setConfirmDeleteId(id);
        return false;
      }
      return true;
    },
    [confirmDeleteId],
  );

  const changeStatus = useCallback(
    async (artifact: MeetingArtifact, status: ArtifactStatus) => {
      setPendingId(artifact.id);
      try {
        const updated = await setArtifactStatus(artifact.id, status);
        setArtifacts((prev) =>
          prev.map((item) => (item.id === updated.id ? updated : item)),
        );
      } catch {
      // Status change is best-effort; leave the row unchanged on failure.
      } finally {
        setPendingId(null);
      }
    },
    [],
  );

  const startEdit = useCallback((artifact: MeetingArtifact) => {
    setAddingKind(null);
    setConfirmDeleteId(null);
    setEditingId(artifact.id);
    setEditDraft({
      text: artifact.text,
      owner: artifact.owner ?? "",
      due: artifact.due ?? "",
    });
  }, []);

  const saveEdit = useCallback(async () => {
    if (!editingId) return;
    const text = editDraft.text.trim();
    if (!text) return;
    const oldId = editingId;
    setPendingId(oldId);
    try {
      const updated = await updateMeetingArtifact(
        oldId,
        text,
        editDraft.owner.trim() || null,
        editDraft.due.trim() || null,
      );
      setArtifacts((prev) =>
        prev.map((item) => (item.id === oldId ? updated : item)),
      );
      setEditingId(null);
      setEditDraft(emptyArtifactDraft());
    } catch {
      showToast("error", "Couldn't update artifact");
    } finally {
      setPendingId(null);
    }
  }, [editingId, editDraft, showToast]);

  const saveAdd = useCallback(async () => {
    if (!addingKind) return;
    const text = addDraft.text.trim();
    if (!text) return;
    setPendingId(`add:${addingKind}`);
    try {
      const created = await createMeetingArtifact(
        meeting.id,
        addingKind,
        text,
        addDraft.owner.trim() || null,
        addDraft.due.trim() || null,
      );
      setArtifacts((prev) => [...prev, created]);
      setAddingKind(null);
      setAddDraft(emptyArtifactDraft());
    } catch {
      showToast("error", "Couldn't add artifact");
    } finally {
      setPendingId(null);
    }
  }, [addingKind, addDraft, meeting.id, showToast]);

  const requestDelete = useCallback(
    async (artifact: MeetingArtifact) => {
      if (!armOrConfirmDelete(artifact.id)) return;
      setPendingId(artifact.id);
      try {
        const ok = await deleteMeetingArtifact(artifact.id);
        if (ok) {
          setArtifacts((prev) =>
            prev.filter((item) => item.id !== artifact.id),
          );
        }
        setConfirmDeleteId(null);
      } catch {
        showToast("error", "Couldn't delete artifact");
      } finally {
        setPendingId(null);
      }
    },
    [armOrConfirmDelete, showToast],
  );

  const saveEntityEdit = useCallback(async () => {
    if (!editingEntityId) return;
    const name = entityEditDraft.name.trim();
    const kind = entityEditDraft.kind.trim();
    if (!name || !kind) return;
    setPendingId(editingEntityId);
    try {
      const updated = await updateMeetingEntity(
        editingEntityId,
        name,
        kind,
      );
      setEntities((prev) =>
        prev.map((item) => (item.id === updated.id ? updated : item)),
      );
      setEditingEntityId(null);
      setEntityEditDraft(emptyEntityDraft());
    } catch (err) {
      const message =
        typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : String(err);
      if (message.includes("already exists")) {
        showToast("error", `Entity already exists: ${name}`);
      } else {
        showToast("error", "Couldn't update entity");
      }
    } finally {
      setPendingId(null);
    }
  }, [editingEntityId, entityEditDraft, showToast]);

  const saveEntityAdd = useCallback(async () => {
    const name = entityAddDraft.name.trim();
    const kind = entityAddDraft.kind.trim();
    if (!name || !kind) return;
    setPendingId("add:entity");
    try {
      const created = await createMeetingEntity(meeting.id, name, kind);
      setEntities((prev) => [...prev, created]);
      setAddingEntity(false);
      setEntityAddDraft(emptyEntityDraft());
    } catch (err) {
      const message =
        typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : String(err);
      if (message.includes("already exists")) {
        showToast("error", `Entity already exists: ${name}`);
      } else {
        showToast("error", "Couldn't add entity");
      }
    } finally {
      setPendingId(null);
    }
  }, [entityAddDraft, meeting.id, showToast]);

  const requestEntityDelete = useCallback(
    async (entity: MeetingEntity) => {
      if (!armOrConfirmDelete(entity.id)) return;
      setPendingId(entity.id);
      try {
        const ok = await deleteMeetingEntity(entity.id);
        if (ok) {
          setEntities((prev) => prev.filter((item) => item.id !== entity.id));
        }
        setConfirmDeleteId(null);
      } catch {
        showToast("error", "Couldn't delete entity");
      } finally {
        setPendingId(null);
      }
    },
    [armOrConfirmDelete, showToast],
  );

  // Extraction rides on summary generation (Sparkles in the Summary AI dock);
  // this panel only reads the results. Copy adapts to why generation is
  // unavailable (live / no transcript) so the empty state still guides.
  const hasTranscript = (segments?.length ?? 0) > 0;
  const canExtract = meeting.status !== "live" && hasTranscript;
  const totalVisible = visible.length + entities.length;

  const emptyHint = canExtract
    ? "No artifacts yet — open Summary options below to generate, or add one below."
    : meeting.status === "live"
      ? "Artifacts appear after the meeting ends and a summary is generated — or add one below."
      : "No transcript to analyze — add artifacts manually, or capture speech first.";

  return (
    <section
      className="flex max-h-[45%] shrink-0 flex-col border-t border-border/70 bg-card"
      aria-label="Meeting artifacts"
    >
      <header className="flex shrink-0 items-center gap-2 px-3 py-1.5">
        {/* Disclosure-first: keep the toggle on the left. */}
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          aria-label={collapsed ? "Expand artifacts" : "Collapse artifacts"}
          aria-expanded={!collapsed}
          onClick={() => setCollapsed((value) => !value)}
        >
          <ChevronDown
            className={cn("transition-transform", collapsed && "-rotate-90")}
          />
        </Button>
        <Sparkles className="size-3.5 text-muted-foreground" aria-hidden />
        <p className="m-0 flex-1 text-xs font-medium text-foreground">
          Artifacts
          {totalVisible > 0 && (
            <span className="ml-1.5 text-muted-foreground">{totalVisible}</span>
          )}
        </p>
      </header>

      {!collapsed && (
        <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-3">
          <div className="flex flex-col gap-3">
            {totalVisible === 0 && (
              <p className="m-0 text-xs text-muted-foreground">{emptyHint}</p>
            )}

            {SECTIONS.map(({ kind, title, addLabel, Icon }) => {
              const items = visible.filter((artifact) => artifact.kind === kind);
              return (
                <section key={kind} aria-label={title}>
                  <p className="m-0 mb-1 flex items-center gap-1.5 text-[0.7rem] font-medium uppercase tracking-wide text-muted-foreground">
                    <Icon className="size-3" aria-hidden />
                    {title}
                  </p>
                  <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
                    {items.map((artifact) =>
                      editingId === artifact.id ? (
                        <li key={artifact.id}>
                          <ArtifactForm
                            draft={editDraft}
                            onChange={setEditDraft}
                            onSave={() => void saveEdit()}
                            onCancel={resetCurationUi}
                            pending={pendingId === artifact.id}
                            saveLabel="Save"
                          />
                        </li>
                      ) : (
                        <ArtifactItem
                          key={artifact.id}
                          artifact={artifact}
                          pending={pendingId === artifact.id}
                          confirmDelete={confirmDeleteId === artifact.id}
                          onConfirm={() => void changeStatus(artifact, "confirmed")}
                          onUnconfirm={() => void changeStatus(artifact, "proposed")}
                          onDismiss={() => void changeStatus(artifact, "dismissed")}
                          onEdit={() => startEdit(artifact)}
                          onDelete={() => void requestDelete(artifact)}
                          onCitationClick={onCitationClick}
                          describeCitation={describeCitation}
                        />
                      ),
                    )}
                    {addingKind === kind ? (
                      <li>
                        <ArtifactForm
                          draft={addDraft}
                          onChange={setAddDraft}
                          onSave={() => void saveAdd()}
                          onCancel={resetCurationUi}
                          pending={pendingId === `add:${kind}`}
                          saveLabel="Add"
                        />
                      </li>
                    ) : (
                      <li>
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-7 w-full justify-start gap-1 border border-dashed border-border/70 text-xs text-muted-foreground hover:text-foreground"
                          onClick={() => {
                            resetCurationUi();
                            setAddingKind(kind);
                            setAddDraft(emptyArtifactDraft());
                          }}
                        >
                          <Plus className="size-3" aria-hidden />
                          {addLabel}
                        </Button>
                      </li>
                    )}
                  </ul>
                </section>
              );
            })}

            <section aria-label="Mentioned entities">
              <p className="m-0 mb-1 flex items-center gap-1.5 text-[0.7rem] font-medium uppercase tracking-wide text-muted-foreground">
                <Tag className="size-3" aria-hidden />
                Mentioned
              </p>
              {editingEntityId ? (
                <EntityForm
                  draft={entityEditDraft}
                  onChange={setEntityEditDraft}
                  onSave={() => void saveEntityEdit()}
                  onCancel={resetCurationUi}
                  pending={pendingId === editingEntityId}
                  saveLabel="Save"
                />
              ) : (
                <div className="flex flex-wrap gap-1">
                  {entities.map((entity) => (
                    <EntityChip
                      key={entity.id}
                      entity={entity}
                      pending={pendingId === entity.id}
                      confirmDelete={confirmDeleteId === entity.id}
                      onEdit={() => {
                        resetCurationUi();
                        setEditingEntityId(entity.id);
                        setEntityEditDraft({
                          name: entity.name,
                          kind: entity.kind,
                        });
                      }}
                      onDelete={() => void requestEntityDelete(entity)}
                    />
                  ))}
                  {addingEntity ? (
                    <EntityForm
                      draft={entityAddDraft}
                      onChange={setEntityAddDraft}
                      onSave={() => void saveEntityAdd()}
                      onCancel={resetCurationUi}
                      pending={pendingId === "add:entity"}
                      saveLabel="Add"
                    />
                  ) : (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      aria-label="Add entity"
                      className="h-6 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
                      onClick={() => {
                        resetCurationUi();
                        setAddingEntity(true);
                        setEntityAddDraft(emptyEntityDraft());
                      }}
                    >
                      <Plus className="size-3" aria-hidden />
                      Add
                    </Button>
                  )}
                </div>
              )}
            </section>

            <p className="m-0 text-[0.7rem] text-muted-foreground">
              AI-extracted — verify before relying on owners or dues.
            </p>
          </div>
        </div>
      )}
    </section>
  );
}

function ArtifactForm({
  draft,
  onChange,
  onSave,
  onCancel,
  pending,
  saveLabel,
}: {
  draft: ArtifactDraft;
  onChange: (next: ArtifactDraft) => void;
  onSave: () => void;
  onCancel: () => void;
  pending: boolean;
  saveLabel: string;
}) {
  return (
    <div
      className="flex flex-col gap-1.5 rounded-md border border-border/70 bg-background/50 p-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onCancel();
        }
        // Enter saves (plan); Shift+Enter keeps a newline in the textarea.
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          onSave();
        }
      }}
    >
      <Textarea
        value={draft.text}
        onChange={(event) => onChange({ ...draft, text: event.target.value })}
        placeholder="Artifact text"
        rows={2}
        className="min-h-12 text-xs"
        autoFocus
        disabled={pending}
        aria-label="Artifact text"
      />
      <div className="flex gap-1.5">
        <Input
          value={draft.owner}
          onChange={(event) => onChange({ ...draft, owner: event.target.value })}
          placeholder="Owner (optional)"
          className="h-7 text-xs"
          disabled={pending}
          aria-label="Owner"
        />
        <Input
          value={draft.due}
          onChange={(event) => onChange({ ...draft, due: event.target.value })}
          placeholder="Due (optional)"
          className="h-7 text-xs"
          disabled={pending}
          aria-label="Due"
        />
      </div>
      <div className="flex justify-end gap-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 text-xs"
          disabled={pending}
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          type="button"
          size="sm"
          className="h-7 text-xs"
          disabled={pending || !draft.text.trim()}
          onClick={onSave}
        >
          {saveLabel}
        </Button>
      </div>
    </div>
  );
}

function EntityForm({
  draft,
  onChange,
  onSave,
  onCancel,
  pending,
  saveLabel,
}: {
  draft: EntityDraft;
  onChange: (next: EntityDraft) => void;
  onSave: () => void;
  onCancel: () => void;
  pending: boolean;
  saveLabel: string;
}) {
  return (
    <div
      className="flex w-full flex-col gap-1.5 rounded-md border border-border/70 bg-background/50 p-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onCancel();
        }
        if (event.key === "Enter") {
          event.preventDefault();
          onSave();
        }
      }}
    >
      <div className="flex gap-1.5">
        <Input
          value={draft.name}
          onChange={(event) => onChange({ ...draft, name: event.target.value })}
          placeholder="Name"
          className="h-7 text-xs"
          autoFocus
          disabled={pending}
          aria-label="Entity name"
        />
        <Input
          value={draft.kind}
          onChange={(event) => onChange({ ...draft, kind: event.target.value })}
          placeholder="person / system / ticket / project"
          className="h-7 text-xs"
          disabled={pending}
          aria-label="Entity kind"
        />
      </div>
      <div className="flex justify-end gap-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 text-xs"
          disabled={pending}
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          type="button"
          size="sm"
          className="h-7 text-xs"
          disabled={pending || !draft.name.trim() || !draft.kind.trim()}
          onClick={onSave}
        >
          {saveLabel}
        </Button>
      </div>
    </div>
  );
}

type CurationMenuAction = {
  label: string;
  icon: LucideIcon;
  ariaLabel?: string;
  variant?: "default" | "destructive";
  /** Keep the menu open (two-click delete first arm). */
  keepOpen?: boolean;
  onSelect: () => void;
};

/** Compact ⋯ menu shared by artifact rows and Mentioned chips. */
function CurationActionsMenu({
  triggerAriaLabel,
  pending,
  open,
  onOpenChange,
  actions,
  triggerVariant = "secondary",
  triggerClassName,
}: {
  triggerAriaLabel: string;
  pending: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  actions: CurationMenuAction[];
  triggerVariant?: "secondary" | "ghost";
  triggerClassName?: string;
}) {
  return (
    <DropdownMenu modal={false} open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant={triggerVariant}
          size="icon-xs"
          aria-label={triggerAriaLabel}
          disabled={pending}
          className={cn("shadow-sm", triggerClassName)}
        >
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="end"
        className="min-w-[7.25rem] p-0.5"
        onCloseAutoFocus={(event) => event.preventDefault()}
      >
        {actions.map((action) => (
          <DropdownMenuItem
            key={action.ariaLabel ?? action.label}
            variant={action.variant}
            disabled={pending}
            aria-label={action.ariaLabel}
            className="h-7 gap-1.5 px-2 py-0 text-xs [&_svg:not([class*='size-'])]:size-3.5"
            onSelect={(event) => {
              if (action.keepOpen) event.preventDefault();
              action.onSelect();
            }}
          >
            <action.icon />
            {action.label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function EntityChip({
  entity,
  pending,
  confirmDelete,
  onEdit,
  onDelete,
}: {
  entity: MeetingEntity;
  pending: boolean;
  confirmDelete: boolean;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);

  const chip = (
    <span className="group/entity relative inline-flex max-w-full">
      {/*
        True overlay ⋯ (parity with artifact rows): badge stays full-bleed;
        trigger sits absolute and never reserves idle chip width.
      */}
      <Badge variant="secondary" className="gap-1 font-normal">
        {entity.name}
        <span className="text-muted-foreground">{entity.kind}</span>
      </Badge>
      <div
        className={cn(
          "absolute inset-y-0 right-0 z-[1] flex items-center rounded-full bg-gradient-to-l from-secondary from-35% to-transparent pl-5",
          "opacity-0 transition-opacity",
          "group-hover/entity:opacity-100 focus-within:opacity-100",
          "[@media(hover:none)]:opacity-100",
          (menuOpen || confirmDelete) && "opacity-100",
        )}
      >
        <CurationActionsMenu
          triggerAriaLabel={`Entity actions for ${entity.name}`}
          pending={pending}
          open={menuOpen}
          onOpenChange={setMenuOpen}
          triggerVariant="ghost"
          triggerClassName="size-4 shadow-none"
          actions={[
            {
              label: "Rename",
              icon: Pencil,
              onSelect: onEdit,
            },
            {
              label: confirmDelete ? "Confirm" : "Delete",
              icon: Trash2,
              variant: "destructive",
              ariaLabel: confirmDelete
                ? `Confirm delete entity ${entity.name}`
                : `Delete entity ${entity.name}`,
              keepOpen: !confirmDelete,
              onSelect: onDelete,
            },
          ]}
        />
      </div>
    </span>
  );

  if (entity.origin === "manual") {
    return <AppTooltip label="Added manually">{chip}</AppTooltip>;
  }
  return chip;
}

function ArtifactItem({
  artifact,
  pending,
  confirmDelete,
  onConfirm,
  onUnconfirm,
  onDismiss,
  onEdit,
  onDelete,
  onCitationClick,
  describeCitation,
}: {
  artifact: MeetingArtifact;
  pending: boolean;
  confirmDelete: boolean;
  onConfirm: () => void;
  onUnconfirm: () => void;
  onDismiss: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onCitationClick: (citation: SegmentCitation) => void;
  describeCitation?: DescribeCitation;
}) {
  const confirmed = artifact.status === "confirmed";
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <li
      className={cn(
        "group/row relative rounded-md border border-border/70 px-2 py-1.5",
        confirmed && "border-accent/40",
      )}
    >
      {/*
        Actions: overlay ⋯ (no idle padding). Compact menu:
        Edit / Dismiss / two-click Delete.
      */}
      <div
        className={cn(
          "absolute right-0.5 top-1 z-[1] rounded-md bg-gradient-to-l from-background from-40% to-transparent pl-6",
          "opacity-0 transition-opacity",
          "group-hover/row:opacity-100 focus-within:opacity-100",
          "[@media(hover:none)]:opacity-100",
          (menuOpen || confirmDelete) && "opacity-100",
        )}
      >
        <CurationActionsMenu
          triggerAriaLabel="Artifact actions"
          pending={pending}
          open={menuOpen}
          onOpenChange={setMenuOpen}
          actions={[
            { label: "Edit", icon: Pencil, onSelect: onEdit },
            { label: "Dismiss", icon: X, onSelect: onDismiss },
            {
              label: confirmDelete ? "Confirm" : "Delete",
              icon: Trash2,
              variant: "destructive",
              ariaLabel: confirmDelete
                ? "Confirm delete artifact"
                : "Delete artifact",
              keepOpen: !confirmDelete,
              onSelect: onDelete,
            },
          ]}
        />
      </div>

      <div className="flex items-start gap-1.5">
        <AppTooltip label={confirmed ? "Confirmed — undo" : "Confirm"}>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            aria-label={
              confirmed ? "Undo confirm artifact" : "Confirm artifact"
            }
            aria-pressed={confirmed}
            disabled={pending}
            className="mt-0.5 size-5 shrink-0 p-0 hover:bg-transparent"
            onClick={confirmed ? onUnconfirm : onConfirm}
          >
            {confirmed ? (
              <CheckCircle2
                className="size-4 fill-accent/20 text-accent"
                aria-hidden
              />
            ) : (
              <Circle
                className="size-4 text-muted-foreground/60 transition-colors hover:text-accent"
                aria-hidden
              />
            )}
          </Button>
        </AppTooltip>
        {artifact.origin === "manual" ? (
          <AppTooltip label="Added manually">
            <p className="m-0 flex-1 text-xs leading-snug text-foreground">
              {artifact.text}
            </p>
          </AppTooltip>
        ) : (
          <p className="m-0 flex-1 text-xs leading-snug text-foreground">
            {artifact.text}
          </p>
        )}
      </div>

      {(artifact.owner || artifact.due) && (
        <p className="m-0 mt-0.5 pl-[1.625rem] text-[0.7rem] text-muted-foreground">
          {artifact.owner && <span>Owner: {artifact.owner}</span>}
          {artifact.owner && artifact.due && <span> · </span>}
          {artifact.due && <span>Due: {artifact.due}</span>}
        </p>
      )}

      {artifact.citations.length > 0 && (
        <div className="mt-1 flex flex-wrap gap-1 pl-[1.625rem]">
          {artifact.citations.map((citation) => (
            <CitationChip
              key={citation.segmentId}
              citation={citation}
              describe={describeCitation}
              onClick={onCitationClick}
            />
          ))}
        </div>
      )}
    </li>
  );
}
