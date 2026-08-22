import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { Check, Eye, EyeOff, Loader2, Pencil, PlugZap, Trash2, X } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { ButtonGroup } from "@/shared/ui/button-group";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";

export interface SecretApiKeyFieldState {
  dirty: boolean;
}

interface Props {
  label: string;
  configured: boolean;
  locked?: boolean;
  /** When false, omit the visible label (parent section already titles this field). Hint still shows if provided. */
  showLabel?: boolean;
  infoHint?: ReactNode;
  onSave: (apiKey: string) => Promise<void>;
  onTest: (apiKey: string) => Promise<void>;
  onRemove: () => Promise<void>;
  onDirtyChange?: (state: SecretApiKeyFieldState) => void;
  onSaved?: () => void;
}

type TestFeedback = { tone: "ok" | "error"; message: string } | null;

const inputAffixBtnClass = cn(
  "inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground outline-none",
  "hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
  "disabled:pointer-events-none disabled:opacity-50",
);

export default function SecretApiKeyField({
  label,
  configured,
  locked = false,
  showLabel = true,
  infoHint,
  onSave,
  onTest,
  onRemove,
  onDirtyChange,
  onSaved,
}: Props) {
  const [editing, setEditing] = useState(!configured);
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [testFeedback, setTestFeedback] = useState<TestFeedback>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const inputId = useId();

  const dirty = apiKey.trim().length > 0;
  const showForm = !configured || editing;
  const canTest = dirty || configured;
  const visibilityLabel = showKey ? "Hide API key" : "Show API key";
  const testLabel = !canTest ? "Paste a key to test" : "Test connection";
  const canCancel = configured;

  useEffect(() => {
    if (configured) {
      setEditing(false);
      setApiKey("");
      setShowKey(false);
      setTestFeedback(null);
    } else {
      setEditing(true);
    }
  }, [configured]);

  useEffect(() => {
    onDirtyChange?.({ dirty });
  }, [dirty, onDirtyChange]);

  const clearDraft = useCallback(() => {
    setApiKey("");
    setShowKey(false);
    setTestFeedback(null);
  }, []);

  const persistKey = useCallback(async () => {
    const trimmed = apiKey.trim();
    if (!trimmed) return;
    setSaving(true);
    try {
      await onSave(trimmed);
      clearDraft();
      setEditing(false);
      onSaved?.();
    } finally {
      setSaving(false);
    }
  }, [apiKey, clearDraft, onSave, onSaved]);

  const handleTest = async () => {
    setTesting(true);
    setTestFeedback(null);
    try {
      await onTest(apiKey);
      setTestFeedback({
        tone: "ok",
        message: dirty
          ? "Connection OK — click Save to apply"
          : "Connection OK",
      });
    } catch (e) {
      setTestFeedback({
        tone: "error",
        message: e instanceof Error ? e.message : String(e),
      });
    } finally {
      setTesting(false);
    }
  };

  const handleRemove = async () => {
    setRemoving(true);
    try {
      await onRemove();
      clearDraft();
      setEditing(true);
    } finally {
      setRemoving(false);
    }
  };

  const startEditing = () => {
    setEditing(true);
    clearDraft();
    requestAnimationFrame(() => inputRef.current?.focus());
  };

  const handleCancel = () => {
    setEditing(false);
    clearDraft();
  };

  const feedbackClass = (tone: "ok" | "error") =>
    cn(
      "m-0 text-sm",
      tone === "ok" ? "text-success" : "text-destructive",
    );

  const labelRow = (htmlFor?: string, muted = false) => {
    // Parent section/group already owns context when the label is omitted.
    if (!showLabel) return null;
    return (
      <span className="inline-flex items-center gap-1.5">
        <Label
          htmlFor={htmlFor}
          className={cn(
            settingsFieldLabelClass,
            muted && "text-muted-foreground",
          )}
        >
          {label}
        </Label>
        {infoHint}
      </span>
    );
  };

  return (
    <div className="flex flex-col gap-3">
      {configured && !editing && (
        <>
          <div className="flex min-w-0 items-center gap-2">
            <Input
              readOnly
              value={"•".repeat(32)}
              aria-label="API key saved"
              className="min-w-0 flex-1 cursor-default font-mono tracking-widest"
              tabIndex={-1}
            />
            <ButtonGroup aria-label="API key actions" className="shrink-0">
              <AppTooltip label={removing ? "Removing…" : "Remove key"}>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  disabled={locked || removing}
                  aria-label={removing ? "Removing…" : "Remove key"}
                  onClick={() => void handleRemove()}
                >
                  {removing ? (
                    <Loader2 className="size-3.5 animate-spin" aria-hidden />
                  ) : (
                    <Trash2 className="size-3.5" aria-hidden />
                  )}
                </Button>
              </AppTooltip>
              <AppTooltip label="Change key">
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  disabled={locked}
                  aria-label="Change key"
                  onClick={startEditing}
                >
                  <Pencil className="size-3.5" aria-hidden />
                </Button>
              </AppTooltip>
              <AppTooltip label={testing ? "Testing…" : "Test connection"}>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  disabled={locked || testing}
                  aria-label={testing ? "Testing…" : "Test connection"}
                  onClick={() => void handleTest()}
                >
                  {testing ? (
                    <Loader2 className="size-3.5 animate-spin" aria-hidden />
                  ) : (
                    <PlugZap className="size-3.5" aria-hidden />
                  )}
                </Button>
              </AppTooltip>
            </ButtonGroup>
          </div>
          {testFeedback && (
            <p className={feedbackClass(testFeedback.tone)} role="status">
              {testFeedback.message}
            </p>
          )}
        </>
      )}

      {showForm && (
        <div className="flex flex-col gap-2">
          {labelRow(inputId)}
          <div className="flex min-w-0 items-center gap-2">
            <div className="relative min-w-0 flex-1">
              <Input
                ref={inputRef}
                id={inputId}
                type={showKey ? "text" : "password"}
                placeholder="Paste your API key"
                value={apiKey}
                autoComplete="off"
                spellCheck={false}
                disabled={locked}
                aria-label={label}
                className="pr-9 font-mono [&::-ms-reveal]:hidden [&::-ms-clear]:hidden"
                onChange={(e) => {
                  setApiKey(e.target.value);
                  setTestFeedback(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && dirty && !saving && !locked) {
                    e.preventDefault();
                    void persistKey();
                  }
                }}
              />
              <div className="absolute top-1/2 right-1 flex -translate-y-1/2 items-center">
                <AppTooltip label={visibilityLabel}>
                  <button
                    type="button"
                    className={inputAffixBtnClass}
                    disabled={locked}
                    aria-label={visibilityLabel}
                    aria-pressed={showKey}
                    onClick={() => setShowKey((visible) => !visible)}
                  >
                    {showKey ? (
                      <EyeOff className="size-3.5" aria-hidden />
                    ) : (
                      <Eye className="size-3.5" aria-hidden />
                    )}
                  </button>
                </AppTooltip>
              </div>
            </div>
            <ButtonGroup aria-label="API key form actions" className="shrink-0">
              <AppTooltip label={testLabel}>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  disabled={locked || testing || !canTest}
                  aria-label={testing ? "Testing…" : testLabel}
                  onClick={() => void handleTest()}
                >
                  {testing ? (
                    <Loader2 className="size-3.5 animate-spin" aria-hidden />
                  ) : (
                    <PlugZap className="size-3.5" aria-hidden />
                  )}
                </Button>
              </AppTooltip>
              <AppTooltip
                label={
                  saving ? "Saving…" : dirty ? "Save" : "Paste a key to save"
                }
              >
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  disabled={locked || saving || !dirty}
                  aria-label={
                    saving ? "Saving…" : dirty ? "Save" : "Paste a key to save"
                  }
                  onClick={() => void persistKey()}
                >
                  {saving ? (
                    <Loader2 className="size-3.5 animate-spin" aria-hidden />
                  ) : (
                    <Check className="size-3.5" aria-hidden />
                  )}
                </Button>
              </AppTooltip>
              <AppTooltip
                label={canCancel ? "Cancel" : "Nothing to cancel"}
              >
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  disabled={locked || !canCancel}
                  aria-label={canCancel ? "Cancel" : "Nothing to cancel"}
                  onClick={handleCancel}
                >
                  <X className="size-3.5" aria-hidden />
                </Button>
              </AppTooltip>
            </ButtonGroup>
          </div>
          {testFeedback && (
            <p className={feedbackClass(testFeedback.tone)} role="status">
              {testFeedback.message}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
