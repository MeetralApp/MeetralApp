import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SecretApiKeyField from "@/shared/components/SecretApiKeyField";
import SettingsField from "@/shared/components/SettingsField";
import SettingsGroup from "@/shared/components/SettingsGroup";
import SectionHeading from "@/shared/components/SectionHeading";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { Checkbox } from "@/shared/ui/checkbox";
import { Label } from "@/shared/ui/label";
import { Button } from "@/shared/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/shared/ui/collapsible";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";

import type { ToastType } from "@/shared/context/toastTypes";
import { loadAiCatalog, useAiCatalog } from "@/features/ai/hooks/useAiCatalog";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import { cleanPayload } from "@/shared/lib/contextPayload";
import {
  emptySonioxContextPayload,
  type MeetingContextPayload,
} from "@/shared/lib/types/pipeline";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import ContextPayloadEditor from "@/shared/components/context-editor/ContextPayloadEditor";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { AiModelInfo } from "@/features/ai/lib/aiTypes";
import {
  resolveLlmSelection,
  resolveSummaryProvider,
  selectionUsable,
  type SummaryProvider,
} from "@/features/config/lib/summaryProvider";
import CustomLlmProfilesSettings from "@/features/config/components/CustomLlmProfilesSettings";
import { listSummaryLanguages } from "@/features/meeting/library/lib/meetingApi";
import type { SummaryLanguageInfo } from "@/features/meeting/detail/lib/summaryTypes";

interface Props {
  config: ConfigView;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTestApiKey: (apiKey: string, provider: AiProvider) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  /** When key is configured, parent chip controls whether manage UI is visible. */
  apiKeyPanelOpen?: boolean;
  onApiKeyDirty?: (dirty: boolean) => void;
  /** Trailing control on the Provider group header (Ready → manage API key). */
  apiKeyChip?: ReactNode;
}

/**
* App-level meeting context for summaries — domain / terminology /
* background injected into summary prompts. Draft + explicit Save so typing
* never spams save_config.
*/
function MeetingContextSection({
  config,
  saving,
  persist,
}: {
  config: ConfigView;
  saving: boolean;
  persist: (patch: Partial<ConfigView>, successMessage: string) => Promise<void>;
}) {
  const saved = useMemo<MeetingContextPayload>(
    () => config.meetingContext ?? emptySonioxContextPayload(),
    [config.meetingContext],
  );
  const savedKey = useMemo(() => JSON.stringify(saved), [saved]);
  const [draft, setDraft] = useState<MeetingContextPayload>(saved);
  const [resetKey, setResetKey] = useState("saved");
  const [resetSeq, setResetSeq] = useState(0);

  // Re-seed only when the persisted value changes (save / external reload) —
  // never on local keystrokes.
  useEffect(() => {
    setDraft(saved);
    setResetKey(savedKey);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- savedKey gates sync
  }, [savedKey]);

  const dirty = useMemo(
    () =>
      JSON.stringify(cleanPayload(draft)) !== JSON.stringify(cleanPayload(saved)),
    [draft, saved],
  );

  return (
    <Collapsible className="group flex flex-col gap-3">
      <div className="flex w-full items-center gap-1.5">
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className="flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-2 rounded-md text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <SectionHeading as="h4">Meeting context</SectionHeading>
            <ChevronDown className="size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
          </button>
        </CollapsibleTrigger>
        <SettingInfoHint label="About meeting context">
          Domain, terminology, and background applied to meeting summaries.
          The model uses them to interpret jargon and repair live-translation
          mistakes (e.g. keep &quot;Sprint planning&quot; as-is in a software
          context). Separate from the Soniox live context, which steers
          transcription during the meeting.
        </SettingInfoHint>
      </div>
      <CollapsibleContent className="flex flex-col gap-3">
        <ContextPayloadEditor
          value={draft}
          onChange={setDraft}
          idPrefix="meeting-context"
          resetKey={resetKey}
          generalTitle="Domain & facts"
        />
        {dirty ? (
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="cursor-pointer"
              disabled={saving}
              onClick={() => {
                setDraft(saved);
                setResetSeq((n) => n + 1);
                setResetKey(`cancel-${resetSeq + 1}`);
              }}
            >
              Cancel
            </Button>
            <Button
              type="button"
              size="sm"
              className="cursor-pointer"
              disabled={saving}
              onClick={() =>
                void persist(
                  { meetingContext: cleanPayload(draft) },
                  "Meeting context saved",
                )
              }
            >
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        ) : null}
      </CollapsibleContent>
    </Collapsible>
  );
}

function ModelInfoHint({ model }: { model: AiModelInfo }) {
  return (
    <SettingInfoHint label={`About ${model.label}`}>
      <p className="m-0 font-medium">{model.label}</p>
      <p className="m-0 mt-1 text-background/80">{model.description}</p>
      <p className="m-0 mt-1 font-mono text-[10px] text-background/60">
        {model.id}
      </p>
    </SettingInfoHint>
  );
}

function FeatureRow({
  id,
  label,
  hint,
  checked,
  disabled,
  onToggle,
}: {
  id: string;
  label: string;
  hint: string;
  checked: boolean;
  disabled?: boolean;
  onToggle: (next: boolean) => void;
}) {
  return (
    <label
      htmlFor={id}
      className="flex cursor-pointer items-start gap-2.5 rounded-md px-1 py-1.5 transition-colors hover:bg-muted/50"
    >
      <Checkbox
        id={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(value) => onToggle(value === true)}
        className="mt-0.5"
      />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm leading-snug text-foreground">{label}</span>
        <span className="text-xs leading-snug text-muted-foreground">{hint}</span>
      </span>
    </label>
  );
}

export default function IntelligenceSettings({
  config,
  onSave,
  onTestApiKey,
  onToast,
  apiKeyPanelOpen = false,
  onApiKeyDirty,
  apiKeyChip,
}: Props) {
  const summaryProvider = resolveSummaryProvider(config);
  const selection = resolveLlmSelection(config);
  const isCustom = selection.kind === "custom";
  const catalog = useAiCatalog(summaryProvider);
  const [saving, setSaving] = useState(false);
  const hasKey = selectionUsable(config, selection);
  const showApiKeyField = !isCustom && (!hasKey || apiKeyPanelOpen);
  const customProfiles = config.customLlmProfiles ?? [];

  const [languages, setLanguages] = useState<SummaryLanguageInfo[]>([]);
  useEffect(() => {
    let cancelled = false;
    listSummaryLanguages()
      .then((items) => {
        if (!cancelled) setLanguages(items);
      })
      .catch(() => {
      // Language list is best-effort; the select still works with Match-meeting only.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const selectedModel = useMemo(
    () => catalog.summaryModels.find((m) => m.id === config.summaryModel),
    [catalog.summaryModels, config.summaryModel],
  );

  const persist = useCallback(
    async (patch: Partial<ConfigView>, successMessage: string) => {
      setSaving(true);
      try {
        // meetingContext is options-only in toSavePayload (absent = keep
        // existing on the backend). Spreading it onto config alone drops it.
        const { meetingContext, ...configPatch } = patch;
        await onSave(
          toSavePayload(
            { ...config, ...configPatch },
            meetingContext !== undefined ? { meetingContext } : {},
          ),
        );
        onToast("success", successMessage);
      } catch (e) {
        onToast("error", String(e));
      } finally {
        setSaving(false);
      }
    },
    [config, onSave, onToast],
  );

  const persistProvider = useCallback(
    async (provider: SummaryProvider) => {
      if (selection.kind === "builtin" && provider === selection.provider) {
        return;
      }
      setSaving(true);
      try {
        const nextCatalog = await loadAiCatalog(provider);
        await onSave(
          toSavePayload({
            ...config,
            summaryProvider: provider,
            summaryModel: nextCatalog.defaults.summaryModel,
            // Switching to a built-in clears the custom selection.
            summaryCustomProfileId: null,
          }),
        );
        onToast("success", "Summary provider saved");
      } catch (e) {
        onToast("error", String(e));
      } finally {
        setSaving(false);
      }
    },
    [config, onSave, onToast, selection],
  );

  const persistCustomProfile = useCallback(
    async (profileId: string) => {
      if (selection.kind === "custom" && selection.profile.id === profileId) {
        return;
      }
      setSaving(true);
      try {
        await onSave(
          toSavePayload({ ...config, summaryCustomProfileId: profileId }),
        );
        onToast("success", "Custom provider selected");
      } catch (e) {
        onToast("error", String(e));
      } finally {
        setSaving(false);
      }
    },
    [config, onSave, onToast, selection],
  );

  const persistSummaryModel = useCallback(
    async (modelId: string) => {
      setSaving(true);
      try {
        await onSave(
          toSavePayload({
            ...config,
            summaryProvider,
            summaryModel: modelId,
          }),
        );
        onToast("success", "Summary model saved");
      } catch (e) {
        onToast("error", String(e));
      } finally {
        setSaving(false);
      }
    },
    [config, onSave, onToast, summaryProvider],
  );

  const persistKey = useCallback(
    async (apiKey: string) => {
      if (summaryProvider === "openAi") {
        await onSave(toSavePayload(config, { openaiApiKey: apiKey }));
        onToast("success", "OpenAI API key saved");
      } else {
        await onSave(toSavePayload(config, { geminiApiKey: apiKey }));
        onToast("success", "Gemini API key saved");
      }
    },
    [config, onSave, onToast, summaryProvider],
  );

  const removeKey = useCallback(async () => {
    if (summaryProvider === "openAi") {
      await onSave(toSavePayload(config, { clearOpenaiApiKey: true }));
      onToast("success", "OpenAI API key removed");
    } else {
      await onSave(toSavePayload(config, { clearGeminiApiKey: true }));
      onToast("success", "Gemini API key removed");
    }
  }, [config, onSave, onToast, summaryProvider]);

  const answerLanguage = config.answerLanguage ?? "";

  return (
    <div className="flex flex-col gap-5">
      <SettingsGroup
        title="Provider & model"
        titleHint={
          <SettingInfoHint label="About intelligence provider">
            Powers meeting summaries. Keys are shared with Translate when that
            engine uses the same provider.
          </SettingInfoHint>
        }
        headerEnd={apiKeyChip}
      >
        <Select
          value={
            selection.kind === "custom"
              ? `custom:${selection.profile.id}`
              : selection.provider
          }
          disabled={saving}
          onValueChange={(value) => {
            if (value.startsWith("custom:")) {
              void persistCustomProfile(value.slice("custom:".length));
            } else {
              void persistProvider(value as SummaryProvider);
            }
          }}
        >
          <SelectTrigger id="settings-summary-provider" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="gemini">Gemini</SelectItem>
            <SelectItem value="openAi">OpenAI</SelectItem>
            {customProfiles.map((profile) => (
              <SelectItem key={profile.id} value={`custom:${profile.id}`}>
                {profile.label} (custom)
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        {isCustom ? (
          <div className="flex flex-col gap-1 rounded-md border border-border/60 px-3 py-2.5">
            <span className="text-sm font-medium text-foreground">
              {selection.profile.label}
            </span>
            <span className="font-mono text-xs text-muted-foreground">
              {selection.profile.baseUrl}
            </span>
            <span className="text-xs text-muted-foreground">
              Model: {selection.profile.chatModel}
            </span>
            <span className="text-xs text-muted-foreground">
              Edit this provider in "Custom providers" below.
            </span>
          </div>
        ) : null}

        {showApiKeyField ? (
          <div
            id="summaries-api-key-panel"
            className={hasKey ? "flex flex-col gap-3 pt-3" : undefined}
          >
            <SecretApiKeyField
              label="API key"
              showLabel={!hasKey}
              configured={hasKey}
              onSave={persistKey}
              onTest={(key) => onTestApiKey(key, summaryProvider)}
              onRemove={removeKey}
              onDirtyChange={({ dirty }) => onApiKeyDirty?.(dirty)}
            />
          </div>
        ) : null}

        {!isCustom ? (
          <SettingsField
            labelContent={
              <span className="inline-flex items-center gap-1.5">
                <Label
                  htmlFor="settings-summary-model"
                  className={settingsFieldLabelClass}
                >
                  Model
                </Label>
                {selectedModel ? (
                  <ModelInfoHint model={selectedModel} />
                ) : (
                  <SettingInfoHint label="About summary model">
                    {hasKey
                      ? "Shared model for meeting summaries."
                      : "Add an API key above to choose a model."}
                  </SettingInfoHint>
                )}
              </span>
            }
          >
            <Select
              value={config.summaryModel}
              disabled={saving || !hasKey}
              onValueChange={(modelId) => void persistSummaryModel(modelId)}
            >
              <SelectTrigger id="settings-summary-model" className="w-full">
                <SelectValue
                  placeholder={
                    hasKey ? "Select summary model" : "Add an API key first"
                  }
                />
              </SelectTrigger>
              <SelectContent position="popper" className="max-h-60">
                {catalog.summaryModels.map((model) => (
                  <SelectItem key={model.id} value={model.id}>
                    {model.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </SettingsField>
        ) : null}

        <SettingsField
          labelContent={
            <span className="inline-flex items-center gap-1.5">
              <Label
                htmlFor="settings-answer-language"
                className={settingsFieldLabelClass}
              >
                AI answer language
              </Label>
              <SettingInfoHint label="About AI answer language">
                Language used by meeting summaries. "Match meeting" follows each
                meeting's You language.
              </SettingInfoHint>
            </span>
          }
        >
          <Select
            value={answerLanguage === "" ? "__match__" : answerLanguage}
            disabled={saving}
            onValueChange={(value) =>
              void persist(
                { answerLanguage: value === "__match__" ? "" : value },
                "AI answer language saved",
              )
            }
          >
            <SelectTrigger id="settings-answer-language" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent position="popper" className="max-h-60">
                <SelectItem value="__match__">Match meeting language</SelectItem>
              {languages.map((lang) => (
                <SelectItem key={lang.code} value={lang.code}>
                  {lang.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </SettingsField>
      </SettingsGroup>

      <CustomLlmProfilesSettings config={config} onToast={onToast} />

      <MeetingContextSection config={config} saving={saving} persist={persist} />

      <SettingsGroup
        title="Features"
        titleHint={
          <SettingInfoHint label="About intelligence features">
            Turn off a surface to hide it and reject its requests. Turning a
            feature back on restores it instantly — no data is deleted.
          </SettingInfoHint>
        }
      >
        <FeatureRow
          id="intelligence-artifacts"
          label="Artifacts"
          hint="Decisions, action items, and entities extracted from the meeting."
          checked={config.artifactsEnabled ?? true}
          disabled={saving}
          onToggle={(next) =>
            void persist({ artifactsEnabled: next }, "Artifacts setting saved")
          }
        />
      </SettingsGroup>
    </div>
  );
}
