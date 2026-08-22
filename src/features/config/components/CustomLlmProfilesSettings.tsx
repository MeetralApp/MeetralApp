import { useCallback, useMemo, useState } from "react";
import { Loader2, Pencil, PlugZap, Plus, Trash2 } from "lucide-react";

import ConfirmDialog from "@/shared/components/ConfirmDialog";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SettingsGroup from "@/shared/components/SettingsGroup";
import SettingsIconButton from "@/shared/components/SettingsIconButton";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";

import type { ToastType } from "@/shared/context/toastTypes";
import {
  deleteCustomLlmProfile,
  testCustomLlmProfile,
  upsertCustomLlmProfile,
} from "@/shared/lib/api/configApi";
import type {
  ConfigView,
  CustomLlmProfileInput,
  CustomLlmProfileView,
} from "@/shared/lib/types/pipeline";

interface Props {
  config: ConfigView;
  onToast: (type: ToastType, text: string) => void;
}

interface FormState {
  id?: string;
  label: string;
  baseUrl: string;
  chatModel: string;
  apiKey: string;
}

const EMPTY_FORM: FormState = {
  label: "",
  baseUrl: "",
  chatModel: "",
  apiKey: "",
};

type TestState =
  | { status: "idle" }
  | { status: "testing" }
  | { status: "ok" }
  | { status: "error"; message: string };

function formToInput(form: FormState): CustomLlmProfileInput {
  return {
    id: form.id,
    label: form.label,
    baseUrl: form.baseUrl,
    chatModel: form.chatModel,
    // Create: only send a key when the user typed one (empty = no auth).
    // Edit: undefined keeps the stored key; leave empty to keep on edit.
    apiKey: form.apiKey.trim() ? form.apiKey.trim() : undefined,
  };
}

export default function CustomLlmProfilesSettings({ config, onToast }: Props) {
  const profiles = useMemo(
    () => config.customLlmProfiles ?? [],
    [config.customLlmProfiles],
  );
  const selectedId = config.summaryCustomProfileId ?? null;

  const [formOpen, setFormOpen] = useState(false);
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [testState, setTestState] = useState<TestState>({ status: "idle" });
  const [saving, setSaving] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<CustomLlmProfileView | null>(
    null,
  );

  const editing = Boolean(form.id);
  const editingKeyConfigured =
    editing &&
    Boolean(
      profiles.find((p) => p.id === form.id)?.apiKeyConfigured,
    );

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setTestState({ status: "idle" });
    setFormOpen(true);
  }, []);

  const openEdit = useCallback((profile: CustomLlmProfileView) => {
    setForm({
      id: profile.id,
      label: profile.label,
      baseUrl: profile.baseUrl,
      chatModel: profile.chatModel,
      apiKey: "",
    });
    setTestState({ status: "idle" });
    setFormOpen(true);
  }, []);

  const patchForm = useCallback((patch: Partial<FormState>) => {
    setForm((prev) => ({ ...prev, ...patch }));
    // Any field change invalidates a previous test result.
    setTestState({ status: "idle" });
  }, []);

  const runTest = useCallback(async () => {
    setTestState({ status: "testing" });
    try {
      await testCustomLlmProfile(formToInput(form));
      setTestState({ status: "ok" });
    } catch (e) {
      setTestState({ status: "error", message: String(e) });
    }
  }, [form]);

  const runSave = useCallback(async () => {
    setSaving(true);
    try {
      // Save runs the same mandatory test-call server-side; a failure
      // rejects the save with a clear message.
      const view = await upsertCustomLlmProfile(formToInput(form));
      if (view.jsonMode === false) {
        onToast(
          "success",
          editing
            ? "Provider updated — JSON mode unsupported; using prompt-only JSON"
            : "Provider added — JSON mode unsupported; using prompt-only JSON",
        );
      } else {
        onToast(
          "success",
          editing ? "Provider profile updated" : "Provider profile added",
        );
      }
      setFormOpen(false);
      setForm(EMPTY_FORM);
      setTestState({ status: "idle" });
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setSaving(false);
    }
  }, [form, editing, onToast]);

  const runDelete = useCallback(async () => {
    if (!deleteTarget) return;
    try {
      await deleteCustomLlmProfile(deleteTarget.id);
      onToast("success", `Removed "${deleteTarget.label}"`);
      setDeleteTarget(null);
    } catch (e) {
      onToast("error", String(e));
    }
  }, [deleteTarget, onToast]);

  const canSubmit =
    form.label.trim() !== "" &&
    form.baseUrl.trim() !== "" &&
    form.chatModel.trim() !== "" &&
    !saving &&
    testState.status !== "testing";

  return (
    <SettingsGroup
      title="Custom providers"
      titleHint={
        <SettingInfoHint label="About custom providers">
          Connect any OpenAI-compatible server (Ollama, LM Studio, OpenRouter,
          vLLM, …) by URL. The connection is tested before a profile can be
          saved — the test checks chat, JSON mode, and streaming.
        </SettingInfoHint>
      }
      headerEnd={
        formOpen ? undefined : (
          <SettingsIconButton
            label="Add custom provider"
            icon={Plus}
            onClick={openCreate}
          />
        )
      }
    >
      {profiles.length === 0 && !formOpen ? (
        <p className="m-0 text-sm text-muted-foreground">
          No custom providers yet. Add one to run meeting summaries against
          your own OpenAI-compatible endpoint.
        </p>
      ) : null}

      {profiles.map((profile) => (
        <div
          key={profile.id}
          className="flex items-start justify-between gap-3 rounded-md border border-border/60 px-3 py-2.5"
        >
          <div className="flex min-w-0 flex-col gap-0.5">
            <span className="truncate text-sm font-medium text-foreground">
              {profile.label}
              {selectedId === profile.id ? (
                <span className="ml-2 rounded bg-primary/15 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-primary">
                  In use
                </span>
              ) : null}
            </span>
            <span className="truncate font-mono text-xs text-muted-foreground">
              {profile.baseUrl}
            </span>
            <span className="truncate text-xs text-muted-foreground">
              {profile.chatModel}
            </span>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={`Edit ${profile.label}`}
              onClick={() => openEdit(profile)}
            >
              <Pencil size={15} aria-hidden />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={`Delete ${profile.label}`}
              onClick={() => setDeleteTarget(profile)}
            >
              <Trash2 size={15} aria-hidden />
            </Button>
          </div>
        </div>
      ))}

      {formOpen ? (
        <form
          className="flex min-w-0 flex-col gap-3 rounded-md border border-border/60 px-3 py-3"
          autoComplete="off"
          onSubmit={(event) => {
            event.preventDefault();
            void runSave();
          }}
        >
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="custom-llm-label" className={settingsFieldLabelClass}>
              Name
            </Label>
            <Input
              id="custom-llm-label"
              name="custom-llm-label"
              value={form.label}
              onChange={(e) => patchForm({ label: e.target.value })}
              placeholder="Ollama (local)"
              autoComplete="off"
              autoFocus
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="inline-flex items-center gap-1.5">
              <Label
                htmlFor="custom-llm-base-url"
                className={settingsFieldLabelClass}
              >
                Server URL
              </Label>
              <SettingInfoHint label="About server URL">
                OpenAI-compatible endpoint. Both forms work:
                http://localhost:11434 or http://localhost:20128/v1 — the
                /v1 prefix is detected and never duplicated.
              </SettingInfoHint>
            </span>
            <Input
              id="custom-llm-base-url"
              name="custom-llm-base-url"
              value={form.baseUrl}
              onChange={(e) => patchForm({ baseUrl: e.target.value })}
              placeholder="http://localhost:11434"
              inputMode="url"
              spellCheck={false}
              // Chromium "Saved info" ignores plain off for URL-like fields;
              // a non-standard token disables the suggestion popup.
              autoComplete="one-time-code"
              data-1p-ignore
              data-lpignore="true"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="inline-flex items-center gap-1.5">
              <Label
                htmlFor="custom-llm-chat-model"
                className={settingsFieldLabelClass}
              >
                Chat model
              </Label>
              <SettingInfoHint label="About chat model">
                Exact model name your server serves. Typos are caught by the
                connection test.
              </SettingInfoHint>
            </span>
            <Input
              id="custom-llm-chat-model"
              name="custom-llm-chat-model"
              value={form.chatModel}
              onChange={(e) => patchForm({ chatModel: e.target.value })}
              placeholder="qwen3:8b"
              spellCheck={false}
              autoComplete="off"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="inline-flex items-center gap-1.5">
              <Label
                htmlFor="custom-llm-api-key"
                className={settingsFieldLabelClass}
              >
                API key (optional)
              </Label>
              <SettingInfoHint label="About API key">
                Only needed if your server requires authentication — most
                local servers (Ollama, LM Studio) do not. Stored in the OS
                keychain, never in plain text. When editing, leave blank to
                keep the saved key.
              </SettingInfoHint>
            </span>
            {editingKeyConfigured && !form.apiKey.trim() ? (
              <p
                className="m-0 text-xs text-muted-foreground"
                data-testid="custom-llm-key-saved"
              >
                API key saved — leave blank to keep, or paste a new key to
                replace it.
              </p>
            ) : null}
            <Input
              id="custom-llm-api-key"
              name="custom-llm-api-key"
              type="password"
              value={form.apiKey}
              onChange={(e) => patchForm({ apiKey: e.target.value })}
              placeholder={
                editingKeyConfigured
                  ? "Leave empty to keep the saved key"
                  : editing
                    ? "Leave empty to keep the saved key"
                    : "Leave empty if your server needs none"
              }
              autoComplete="new-password"
              data-1p-ignore
              data-lpignore="true"
            />
          </div>

          {testState.status === "ok" ? (
            <p className="m-0 min-w-0 break-words text-sm text-success" role="status">
              Connection OK — chat, JSON mode, and streaming all work.
            </p>
          ) : null}
          {testState.status === "error" ? (
            <div
              className="min-w-0 max-h-36 overflow-x-hidden overflow-y-auto rounded-md border border-destructive/30 bg-destructive/5 px-2.5 py-2 text-sm leading-snug text-destructive break-words [overflow-wrap:anywhere]"
              role="alert"
            >
              {testState.message}
            </div>
          ) : null}

          <div className="flex items-center justify-end gap-2 pt-1">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={saving || testState.status === "testing"}
              onClick={() => {
                setFormOpen(false);
                setForm(EMPTY_FORM);
                setTestState({ status: "idle" });
              }}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled={!canSubmit}
              onClick={() => void runTest()}
            >
              {testState.status === "testing" ? (
                <>
                  <Loader2 size={14} className="animate-spin" aria-hidden />
                  Testing…
                </>
              ) : (
                <>
                  <PlugZap size={14} aria-hidden />
                  Test connection
                </>
              )}
            </Button>
            <Button type="submit" size="sm" disabled={!canSubmit}>
              {saving ? (
                <>
                  <Loader2 size={14} className="animate-spin" aria-hidden />
                  Saving…
                </>
              ) : editing ? (
                "Save changes"
              ) : (
                "Add provider"
              )}
            </Button>
          </div>
        </form>
      ) : null}

      <ConfirmDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null);
        }}
        title={`Delete "${deleteTarget?.label ?? ""}"?`}
        description={
          selectedId === deleteTarget?.id
            ? "This provider is currently selected for summaries. Deleting it switches back to the built-in provider. Its API key is removed from the keychain."
            : "The profile and its API key are removed. Meetings already summarized are not affected."
        }
        confirmLabel="Delete"
        confirmingLabel="Deleting…"
        destructive
        onConfirm={runDelete}
      />
    </SettingsGroup>
  );
}
