import { useCallback, useEffect, useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, Plus } from "lucide-react";

import SonioxContextPayloadEditor from "@/shared/components/context-editor/ContextPayloadEditor";
import ConfirmDialog from "@/shared/components/ConfirmDialog";
import SettingsGroup from "@/shared/components/SettingsGroup";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Checkbox } from "@/shared/ui/checkbox";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";

import type { ToastType } from "@/shared/context/toastTypes";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import {
  cleanPayload,
  newSonioxProfile,
  payloadSummary,
} from "@/features/voice/lib/sonioxContext";
import {
  emptySonioxContextPayload,
  estimateSonioxPayloadChars,
  mergeSonioxContextPreview,
  SONIOX_CONTEXT_CHAR_BUDGET,
  type SonioxContextPayload,
  type SonioxContextProfile,
} from "@/shared/lib/types/pipeline";

interface Props {
  config: ConfigView;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
  onDirtyChange?: (dirty: boolean) => void;
}

type EditTarget =
  | { kind: "alwaysOn" }
  | { kind: "profile"; profileId: string; isNew?: boolean };

const EMPTY_SONIOX_PROFILES: SonioxContextProfile[] = [];

export default function SonioxContextSettings({
  config,
  onSave,
  onToast,
  onDirtyChange,
}: Props) {
  const alwaysOn = config.sonioxAlwaysOn ?? emptySonioxContextPayload();
  const profiles = config.sonioxContextProfiles ?? EMPTY_SONIOX_PROFILES;

  const [edit, setEdit] = useState<EditTarget | null>(null);
  const [draftName, setDraftName] = useState("");
  const [draftInclude, setDraftInclude] = useState(true);
  const [draftPayload, setDraftPayload] = useState<SonioxContextPayload>(
    emptySonioxContextPayload(),
  );
  const [draftProfiles, setDraftProfiles] = useState<SonioxContextProfile[] | null>(
    null,
  );
  const [saving, setSaving] = useState(false);
  const [discardOpen, setDiscardOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);

  const workingProfiles = draftProfiles ?? profiles;

  const openAlwaysOn = useCallback(() => {
    setEdit({ kind: "alwaysOn" });
    setDraftPayload(alwaysOn);
    setDraftName("");
    setDraftInclude(true);
  }, [alwaysOn]);

  const openProfile = useCallback(
    (profile: SonioxContextProfile, isNew = false) => {
      setEdit({ kind: "profile", profileId: profile.id, isNew });
      setDraftPayload(profile.payload);
      setDraftName(profile.name);
      setDraftInclude(profile.includeAlwaysOn);
    },
    [],
  );

  const closeEdit = useCallback(() => {
    setEdit(null);
    setDraftProfiles(null);
    onDirtyChange?.(false);
  }, [onDirtyChange]);

  const dirty = useMemo(() => {
    if (!edit) return false;
    if (edit.kind === "alwaysOn") {
      return (
        JSON.stringify(cleanPayload(draftPayload)) !==
        JSON.stringify(cleanPayload(alwaysOn))
      );
    }
    const original = workingProfiles.find((p) => p.id === edit.profileId);
    if (!original) return true;
    return (
      draftName.trim() !== original.name ||
      draftInclude !== original.includeAlwaysOn ||
      JSON.stringify(cleanPayload(draftPayload)) !==
        JSON.stringify(cleanPayload(original.payload))
    );
  }, [alwaysOn, draftInclude, draftName, draftPayload, edit, workingProfiles]);

  useEffect(() => {
    onDirtyChange?.(Boolean(edit) && dirty);
  }, [dirty, edit, onDirtyChange]);

  const aoChars = estimateSonioxPayloadChars(cleanPayload(draftPayload));
  const alwaysOnChars = estimateSonioxPayloadChars(cleanPayload(alwaysOn));

  const budgetPreview = useMemo(() => {
    if (!edit || edit.kind === "alwaysOn") {
      return { ao: aoChars, profile: 0, combined: aoChars };
    }
    const profileDraft: SonioxContextProfile = {
      id: edit.profileId,
      name: draftName,
      includeAlwaysOn: draftInclude,
      payload: cleanPayload(draftPayload),
    };
    const merged = mergeSonioxContextPreview(alwaysOn, profileDraft);
    return {
      ao: alwaysOnChars,
      profile: estimateSonioxPayloadChars(profileDraft.payload),
      combined: estimateSonioxPayloadChars(merged),
    };
  }, [
    alwaysOn,
    alwaysOnChars,
    aoChars,
    draftInclude,
    draftName,
    draftPayload,
    edit,
  ]);

  const overBudget =
    budgetPreview.combined > SONIOX_CONTEXT_CHAR_BUDGET ||
    (edit?.kind === "alwaysOn" && aoChars > SONIOX_CONTEXT_CHAR_BUDGET);

  const persistAlwaysOn = useCallback(async () => {
    if (overBudget) {
      onToast(
        "error",
        `Context is over the ~${SONIOX_CONTEXT_CHAR_BUDGET.toLocaleString()} character budget`,
      );
      return;
    }
    setSaving(true);
    try {
      await onSave(
        toSavePayload(config, {
          sonioxAlwaysOn: cleanPayload(draftPayload),
        }),
      );
      onToast("success", "Always-on context saved");
      closeEdit();
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setSaving(false);
    }
  }, [closeEdit, config, draftPayload, onSave, onToast, overBudget]);

  const persistProfile = useCallback(async () => {
    if (!edit || edit.kind !== "profile") return;
    const name = draftName.trim() || "Untitled profile";
    if (overBudget) {
      onToast(
        "error",
        `Context is over the ~${SONIOX_CONTEXT_CHAR_BUDGET.toLocaleString()} character budget`,
      );
      return;
    }
    const nextProfile: SonioxContextProfile = {
      id: edit.profileId,
      name,
      includeAlwaysOn: draftInclude,
      payload: cleanPayload(draftPayload),
    };
    const nextList = workingProfiles.some((p) => p.id === edit.profileId)
      ? workingProfiles.map((p) => (p.id === edit.profileId ? nextProfile : p))
      : [...workingProfiles, nextProfile];
    setSaving(true);
    try {
      await onSave(
        toSavePayload(config, {
          sonioxContextProfiles: nextList,
        }),
      );
      onToast("success", "Meeting profile saved");
      setDraftProfiles(null);
      closeEdit();
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setSaving(false);
    }
  }, [
    closeEdit,
    config,
    draftInclude,
    draftName,
    draftPayload,
    edit,
    onSave,
    onToast,
    overBudget,
    workingProfiles,
  ]);

  const handleNewProfile = useCallback(() => {
    const profile = newSonioxProfile();
    setDraftProfiles([...profiles, profile]);
    openProfile(profile, true);
  }, [openProfile, profiles]);

  const handleDeleteProfile = useCallback(async () => {
    if (!edit || edit.kind !== "profile") return;
    const profileId = edit.profileId;

    if (edit.isNew) {
      setDeleteOpen(false);
      closeEdit();
      return;
    }

    const nextList = profiles.filter((p) => p.id !== profileId);
    const clearActive =
      config.sonioxActiveContextProfileId === profileId ? "" : undefined;
    try {
      await onSave(
        toSavePayload(config, {
          sonioxContextProfiles: nextList,
          ...(clearActive !== undefined
            ? { sonioxActiveContextProfileId: clearActive }
            : {}),
        }),
      );
      onToast("success", "Profile deleted");
      setDeleteOpen(false);
      closeEdit();
    } catch (e) {
      onToast("error", String(e));
    }
  }, [closeEdit, config, edit, onSave, onToast, profiles]);

  const discardAndClose = useCallback(() => {
    setDiscardOpen(false);
    closeEdit();
  }, [closeEdit]);

  const handleBack = useCallback(() => {
    if (dirty) {
      setDiscardOpen(true);
      return;
    }
    closeEdit();
  }, [closeEdit, dirty]);

  if (edit) {
    const title =
      edit.kind === "alwaysOn"
        ? "Edit Always-on"
        : `Edit: ${draftName.trim() || "Untitled"}`;
    return (
      <>
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="cursor-pointer gap-1 px-2"
            onClick={handleBack}
          >
            <ChevronLeft className="size-4" aria-hidden />
            Back
          </Button>
          <span className="text-sm font-semibold tracking-tight text-foreground">
            {title}
          </span>
        </div>

        {edit.kind === "profile" ? (
          <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="soniox-profile-name" className={settingsFieldLabelClass}>
                Name
              </Label>
              <Input
                id="soniox-profile-name"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                placeholder="e.g. Sprint planning"
              />
            </div>
            <div className="flex items-start gap-2">
              <Checkbox
                id="soniox-include-ao"
                checked={draftInclude}
                onCheckedChange={(checked) =>
                  setDraftInclude(checked === true)
                }
              />
              <span className="inline-flex items-center gap-1.5">
                <Label htmlFor="soniox-include-ao" className="font-normal">
                  Include Always-on
                </Label>
                <SettingInfoHint label="About Include Always-on">
                  When on, Always-on merges with this profile at Start. Turn off
                  for meetings that should not use shared names or glossary.
                </SettingInfoHint>
              </span>
            </div>
          </div>
        ) : null}

        <SonioxContextPayloadEditor
          value={draftPayload}
          onChange={setDraftPayload}
          idPrefix={
            edit.kind === "alwaysOn" ? "soniox-ao" : `soniox-${edit.profileId}`
          }
          resetKey={
            edit.kind === "alwaysOn" ? "always-on" : edit.profileId
          }
        />

        <div className="flex flex-col gap-2">
          <div className="inline-flex flex-wrap items-center gap-1.5">
            <p
              className={cn(
                "m-0 text-xs",
                overBudget ? "text-destructive" : "text-muted-foreground",
              )}
              aria-live="polite"
            >
              {edit.kind === "alwaysOn" ? (
                <>
                  Always-on {budgetPreview.ao.toLocaleString()} /{" "}
                  {SONIOX_CONTEXT_CHAR_BUDGET.toLocaleString()}
                </>
              ) : (
                <>
                  AO {budgetPreview.ao.toLocaleString()} · Profile{" "}
                  {budgetPreview.profile.toLocaleString()} · Combined{" "}
                  {budgetPreview.combined.toLocaleString()} /{" "}
                  {SONIOX_CONTEXT_CHAR_BUDGET.toLocaleString()}
                </>
              )}
              {overBudget ? " — over budget" : ""}
            </p>
            <SettingInfoHint label="About context size">
              Soft budget aligned with Soniox (~10,000 characters). Combined
              size is what Start sends when Include Always-on is on.
            </SettingInfoHint>
          </div>
          {edit.kind === "profile" || dirty ? (
            <div className="flex flex-wrap items-center justify-end gap-2">
              {edit.kind === "profile" && !edit.isNew ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="cursor-pointer border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
                  disabled={saving}
                  onClick={() => setDeleteOpen(true)}
                >
                  Delete
                </Button>
              ) : null}
              {dirty ? (
                <>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="cursor-pointer"
                    disabled={saving}
                    onClick={handleBack}
                  >
                    Cancel
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    className="cursor-pointer"
                    disabled={saving || overBudget}
                    onClick={() =>
                      void (edit.kind === "alwaysOn"
                        ? persistAlwaysOn()
                        : persistProfile())
                    }
                  >
                    {saving ? "Saving…" : "Save"}
                  </Button>
                </>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>

      <ConfirmDialog
        open={discardOpen}
        onOpenChange={setDiscardOpen}
        title="Discard unsaved changes?"
        description="Your edits will be lost."
        confirmLabel="Discard"
        cancelLabel="Keep editing"
        destructive
        onConfirm={discardAndClose}
      />
      <ConfirmDialog
        open={deleteOpen}
        onOpenChange={setDeleteOpen}
        title="Delete this profile?"
        description={
          edit.kind === "profile" ? (
            <>
              <span className="block break-all font-medium text-foreground">
                “{draftName.trim() || "Untitled"}”
              </span>
              <span className="mt-1.5 block">This cannot be undone.</span>
            </>
          ) : undefined
        }
        confirmLabel="Delete"
        destructive
        onConfirm={() => void handleDeleteProfile()}
      />
      </>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <SettingsGroup title="Always-on">
        <button
          type="button"
          className="flex w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-border px-3 py-2.5 text-left transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={openAlwaysOn}
        >
          <span className="min-w-0">
            <span className="block text-sm font-medium text-foreground">
              Edit always-on context
            </span>
            <span className="block text-xs text-muted-foreground">
              {payloadSummary(alwaysOn)}
            </span>
          </span>
          <ChevronRight
            className="size-4 shrink-0 text-muted-foreground"
            aria-hidden
          />
        </button>
      </SettingsGroup>

      <SettingsGroup
        title="Meeting profiles"
        titleHint={
          <SettingInfoHint label="About meeting profiles">
            Reusable templates (Sprint, Tech design, …). Choose the active one
            on Live before Start. Edit here only.
          </SettingInfoHint>
        }
      >
        {workingProfiles.length === 0 ? (
          <p className="m-0 text-xs text-muted-foreground">
            No meeting profiles yet. Create one for each meeting type.
          </p>
        ) : (
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {workingProfiles.map((profile) => (
              <li key={profile.id}>
                <button
                  type="button"
                  className="flex w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-border px-3 py-2.5 text-left transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onClick={() => openProfile(profile)}
                >
                  <span className="min-w-0">
                    <span className="flex items-center gap-1.5">
                      <span className="truncate text-sm font-medium text-foreground">
                        {profile.name}
                      </span>
                      {profile.includeAlwaysOn ? (
                        <Badge
                          variant="outline"
                          className="px-1.5 py-0 text-[0.65rem]"
                        >
                          +AO
                        </Badge>
                      ) : null}
                    </span>
                    <span className="block text-xs text-muted-foreground">
                      {payloadSummary(profile.payload)}
                    </span>
                  </span>
                  <ChevronRight
                    className="size-4 shrink-0 text-muted-foreground"
                    aria-hidden
                  />
                </button>
              </li>
            ))}
          </ul>
        )}

        <Button
          type="button"
          variant="outline"
          className="h-auto cursor-pointer gap-1.5 self-stretch py-2.5"
          onClick={handleNewProfile}
        >
          <Plus className="size-3.5" aria-hidden />
          New profile
        </Button>
      </SettingsGroup>
    </div>
  );
}
