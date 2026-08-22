import {
  useCallback,
  useEffect,
  useId,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { ArrowRight, Plus, Trash2 } from "lucide-react";

import SettingsGroup from "@/shared/components/SettingsGroup";
import AppTooltip from "@/shared/components/AppTooltip";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Textarea } from "@/shared/ui/textarea";
import { cn } from "@/shared/lib/utils";
import {
  ensureGeneralRows,
  ensureTranslationRows,
  presetLabel,
  type IdGeneral,
  type IdPair,
} from "@/shared/lib/contextPayload";
import type { SonioxContextPayload } from "@/shared/lib/types/pipeline";

import KeyCombobox from "./KeyCombobox";
import SectionHeader from "./SectionHeader";
import TermTagsInput from "./TermTagsInput";

/** Meeting context: full-width key + value; select row to delete from header. */
function MeetingContextBlock({
  leftId,
  rightId,
  leftValue,
  rightValue,
  rightPlaceholder,
  leftAriaLabel,
  rightAriaLabel,
  selected,
  onSelect,
  onLeftChange,
  onRightChange,
}: {
  leftId: string;
  rightId: string;
  leftValue: string;
  rightValue: string;
  rightPlaceholder: string;
  leftAriaLabel: string;
  rightAriaLabel: string;
  selected: boolean;
  onSelect: (additive: boolean, keepIfSelected?: boolean) => void;
  onLeftChange: (v: string) => void;
  onRightChange: (v: string) => void;
}) {
  return (
    <div
      role="option"
      aria-selected={selected}
      className={cn(
        "rounded-md border-l-2 px-1 py-1 transition-colors",
        selected
          ? "border-l-primary bg-primary/5"
          : "border-l-transparent hover:bg-muted/40",
      )}
      onClick={(e) => {
        // Ignore clicks inside the key suggestion dropdown (not the parent listbox).
        if ((e.target as HTMLElement).closest("[data-key-suggestions]")) return;
        const onField = Boolean(
          (e.target as HTMLElement).closest(
            "input, textarea, button, [role='combobox']",
          ),
        );
        onSelect(e.metaKey || e.ctrlKey, onField);
      }}
    >
      <div className="flex flex-col gap-2">
        <KeyCombobox
          id={leftId}
          value={leftValue}
          onChange={onLeftChange}
          ariaLabel={leftAriaLabel}
        />
        <Textarea
          id={rightId}
          value={rightValue}
          onChange={(e) => onRightChange(e.target.value)}
          placeholder={
            rightPlaceholder ||
            (presetLabel(leftValue)
              ? `Value for ${presetLabel(leftValue)}`
              : "Value")
          }
          aria-label={rightAriaLabel}
          rows={1}
          className="min-h-8 w-full resize-none py-1.5"
        />
      </div>
    </div>
  );
}

/** How to translate: source → target; select row to delete from header. */
function TranslationPairRow({
  leftId,
  rightId,
  leftValue,
  rightValue,
  leftPlaceholder,
  rightPlaceholder,
  leftAriaLabel,
  rightAriaLabel,
  selected,
  onSelect,
  onLeftChange,
  onRightChange,
}: {
  leftId: string;
  rightId: string;
  leftValue: string;
  rightValue: string;
  leftPlaceholder: string;
  rightPlaceholder: string;
  leftAriaLabel: string;
  rightAriaLabel: string;
  selected: boolean;
  onSelect: (additive: boolean, keepIfSelected?: boolean) => void;
  onLeftChange: (v: string) => void;
  onRightChange: (v: string) => void;
}) {
  const trimmedLeft = leftValue.trim();
  const trimmedRight = rightValue.trim();
  const isSame =
    trimmedLeft.length > 0 && trimmedLeft === trimmedRight;

  return (
    <div
      role="option"
      aria-selected={selected}
      className={cn(
        "flex flex-col gap-1.5 rounded-md border-l-2 px-1 py-1 transition-colors",
        selected
          ? "border-l-primary bg-primary/5"
          : "border-l-transparent hover:bg-muted/40",
      )}
      onClick={(e) => {
        const onField = Boolean(
          (e.target as HTMLElement).closest("input, textarea, button"),
        );
        onSelect(e.metaKey || e.ctrlKey, onField);
      }}
    >
      <div className="flex items-center gap-1.5">
        <Input
          id={leftId}
          value={leftValue}
          onChange={(e) => onLeftChange(e.target.value)}
          placeholder={leftPlaceholder}
          aria-label={leftAriaLabel}
          className="h-8 min-w-0 flex-1"
        />
        <ArrowRight
          className="size-3.5 shrink-0 text-muted-foreground"
          aria-hidden
        />
        <Input
          id={rightId}
          value={rightValue}
          onChange={(e) => onRightChange(e.target.value)}
          placeholder={rightPlaceholder}
          aria-label={rightAriaLabel}
          className="h-8 min-w-0 flex-1"
        />
      </div>
      {isSame ? (
        <Badge
          variant="secondary"
          className="h-5 w-fit px-1.5 text-[0.65rem] font-normal"
        >
          Same — keep unchanged
        </Badge>
      ) : null}
    </div>
  );
}

interface Props {
  value: SonioxContextPayload;
  onChange: (next: SonioxContextPayload) => void;
  idPrefix?: string;
  /** Change to re-seed local editor state from `value` (e.g. profile id). */
  resetKey?: string;
  /** First block title (general key–value facts). */
  generalTitle?: string;
}

/** Shared 4-block context editor (Soniox always-on/profile, Meeting context). */
export default function ContextPayloadEditor({
  value,
  onChange,
  idPrefix = "soniox",
  resetKey = "default",
  generalTitle = "Meeting context",
}: Props) {
  const nextId = useId();
  const [idSeq, setIdSeq] = useState(0);
  const [general, setGeneral] = useState(() =>
    ensureGeneralRows(value.general, `${idPrefix}-g-empty`),
  );
  const [pairs, setPairs] = useState(() =>
    ensureTranslationRows(value.translationTerms, `${idPrefix}-t-empty`),
  );
  const [text, setText] = useState(value.text);
  const [terms, setTerms] = useState(() =>
    value.terms.map((t) => t.trim()).filter(Boolean),
  );
  const [selectedGeneral, setSelectedGeneral] = useState<Set<string>>(
    () => new Set(),
  );
  const [selectedPairs, setSelectedPairs] = useState<Set<string>>(
    () => new Set(),
  );

  useEffect(() => {
    setGeneral(ensureGeneralRows(value.general, `${idPrefix}-g-empty`));
    setPairs(
      ensureTranslationRows(value.translationTerms, `${idPrefix}-t-empty`),
    );
    setText(value.text);
    setTerms(value.terms.map((t) => t.trim()).filter(Boolean));
    setSelectedGeneral(new Set());
    setSelectedPairs(new Set());
  // Only re-seed when opening a different entity — not on each keystroke.
  // eslint-disable-next-line react-hooks/exhaustive-deps -- resetKey gates sync
  }, [resetKey]);

  useEffect(() => {
    if (selectedGeneral.size === 0 && selectedPairs.size === 0) return;
    const onKeyDown = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setSelectedGeneral(new Set());
      setSelectedPairs(new Set());
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedGeneral.size, selectedPairs.size]);

  const emit = useCallback(
    (next: {
      general?: IdGeneral[];
      text?: string;
      terms?: string[];
      pairs?: IdPair[];
    }) => {
      const g = next.general ?? general;
      const t = next.text ?? text;
      const tm = next.terms ?? terms;
      const p = next.pairs ?? pairs;
      onChange({
        general: g.map(({ key, value: v }) => ({ key, value: v })),
        text: t,
        terms: tm.map((row) => row.trim()).filter(Boolean),
        translationTerms: p.map(({ source, target }) => ({ source, target })),
      });
    },
    [general, onChange, pairs, terms, text],
  );

  const toggleSelect = useCallback(
    (
      id: string,
      additive: boolean,
      setter: Dispatch<SetStateAction<Set<string>>>,
      keepIfSelected = false,
    ) => {
      setter((prev) => {
        if (additive) {
          const next = new Set(prev);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        }
        // Clicking a field inside an already-selected block keeps selection.
        if (keepIfSelected && prev.has(id)) return prev;
        // Click again on the only selected block (chrome) → deselect.
        if (prev.size === 1 && prev.has(id)) return new Set();
        return new Set([id]);
      });
    },
    [],
  );

  const deleteSelectedGeneral = useCallback(() => {
    if (selectedGeneral.size === 0) return;
    setIdSeq((n) => n + 1);
    const filtered = general.filter((p) => !selectedGeneral.has(p.id));
    const next =
      filtered.length > 0
        ? filtered
        : [{ id: `${nextId}-g-${idSeq}`, key: "", value: "" }];
    setGeneral(next);
    setSelectedGeneral(new Set());
    emit({ general: next });
  }, [emit, general, idSeq, nextId, selectedGeneral]);

  const deleteSelectedPairs = useCallback(() => {
    if (selectedPairs.size === 0) return;
    setIdSeq((n) => n + 1);
    const filtered = pairs.filter((p) => !selectedPairs.has(p.id));
    const next =
      filtered.length > 0
        ? filtered
        : [{ id: `${nextId}-t-${idSeq}`, source: "", target: "" }];
    setPairs(next);
    setSelectedPairs(new Set());
    emit({ pairs: next });
  }, [emit, idSeq, nextId, pairs, selectedPairs]);

  return (
    <div className="flex flex-col gap-5">
      <SettingsGroup>
        <SectionHeader
          title={generalTitle}
          hintLabel="About meeting context"
          hint={
            <>
              Structured key–value facts. Type a key or pick a suggestion
              (domain, speakers, …). Click a row to select; Ctrl/Cmd+click for
              multi-select, then delete from the header. Keep roughly 10 or
              fewer pairs.
            </>
          }
          action={
            <div className="flex items-center gap-0.5">
              {selectedGeneral.size > 0 ? (
                <AppTooltip
                  label={
                    selectedGeneral.size === 1
                      ? "Delete selected"
                      : `Delete ${selectedGeneral.size} selected`
                  }
                >
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="size-7 cursor-pointer text-destructive hover:text-destructive"
                    aria-label={
                      selectedGeneral.size === 1
                        ? "Delete selected pair"
                        : `Delete ${selectedGeneral.size} selected pairs`
                    }
                    onClick={deleteSelectedGeneral}
                  >
                    <Trash2 className="size-3.5" aria-hidden />
                  </Button>
                </AppTooltip>
              ) : null}
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-7 cursor-pointer"
                aria-label="Add meeting context"
                onClick={() => {
                  setIdSeq((n) => n + 1);
                  const next = [
                    ...general,
                    { id: `${nextId}-g-${idSeq}`, key: "", value: "" },
                  ];
                  setGeneral(next);
                  emit({ general: next });
                }}
              >
                <Plus className="size-3.5" aria-hidden />
              </Button>
            </div>
          }
        />
        <div
          className="flex flex-col gap-2.5"
          role="listbox"
          aria-label="Meeting context pairs"
          aria-multiselectable="true"
        >
          {general.map((pair, index) => (
            <MeetingContextBlock
              key={pair.id}
              leftId={`${pair.id}-key`}
              rightId={`${pair.id}-value`}
              leftValue={pair.key}
              rightValue={pair.value}
              rightPlaceholder={
                pair.key.trim() === "domain"
                  ? "e.g. Healthcare"
                  : pair.key.trim() === "topic"
                    ? "e.g. Q3 planning"
                    : pair.key.trim() === "speakers"
                      ? "e.g. Alice (host), Bob"
                      : "Short value"
              }
              leftAriaLabel={`Key ${index + 1}`}
              rightAriaLabel={`Value ${index + 1}`}
              selected={selectedGeneral.has(pair.id)}
              onSelect={(additive, keepIfSelected) =>
                toggleSelect(
                  pair.id,
                  additive,
                  setSelectedGeneral,
                  keepIfSelected,
                )
              }
              onLeftChange={(v) => {
                const next = general.map((p) =>
                  p.id === pair.id ? { ...p, key: v } : p,
                );
                setGeneral(next);
                emit({ general: next });
              }}
              onRightChange={(v) => {
                const next = general.map((p) =>
                  p.id === pair.id ? { ...p, value: v } : p,
                );
                setGeneral(next);
                emit({ general: next });
              }}
            />
          ))}
        </div>
      </SettingsGroup>

      <SettingsGroup>
        <SectionHeader
          title="Background"
          hintLabel="About background text"
          hint={
            <>
              Free-form notes, prior meeting text, or reference documents. Less
              influential than meeting context or terms; counts toward the
              shared character budget.
            </>
          }
        />
        <Textarea
          id={`${idPrefix}-context-text`}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            emit({ text: e.target.value });
          }}
          placeholder="Optional background for this meeting"
          aria-label="Background"
          rows={3}
          className="min-h-[4.5rem] resize-none"
        />
      </SettingsGroup>

      <SettingsGroup>
        <SectionHeader
          title="Words to recognize"
          hintLabel="About recognition terms"
          hint={
            <>
              Names and jargon that should stay intact. Type a term and press
              Enter or comma to add a tag; Backspace removes the last tag when
              the field is empty.
            </>
          }
        />
        <TermTagsInput
          id={`${idPrefix}-terms`}
          terms={terms}
          onChange={(next) => {
            setTerms(next);
            emit({ terms: next });
          }}
        />
      </SettingsGroup>

      <SettingsGroup>
        <SectionHeader
          title="How to translate"
          hintLabel="About translation terms"
          hint={
            <>
              Preferred source → target pairs for names or ambiguous phrases. To
              keep a name unchanged, set source and target to the same text.
              Click a row to select; Ctrl/Cmd+click for multi-select, then
              delete from the header.
            </>
          }
          action={
            <div className="flex items-center gap-0.5">
              {selectedPairs.size > 0 ? (
                <AppTooltip
                  label={
                    selectedPairs.size === 1
                      ? "Delete selected"
                      : `Delete ${selectedPairs.size} selected`
                  }
                >
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="size-7 cursor-pointer text-destructive hover:text-destructive"
                    aria-label={
                      selectedPairs.size === 1
                        ? "Delete selected pair"
                        : `Delete ${selectedPairs.size} selected pairs`
                    }
                    onClick={deleteSelectedPairs}
                  >
                    <Trash2 className="size-3.5" aria-hidden />
                  </Button>
                </AppTooltip>
              ) : null}
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-7 cursor-pointer"
                aria-label="Add translation pair"
                onClick={() => {
                  setIdSeq((n) => n + 1);
                  const next = [
                    ...pairs,
                    { id: `${nextId}-t-${idSeq}`, source: "", target: "" },
                  ];
                  setPairs(next);
                  emit({ pairs: next });
                }}
              >
                <Plus className="size-3.5" aria-hidden />
              </Button>
            </div>
          }
        />
        <div className="flex flex-col gap-2">
          <p className="m-0 px-0.5 text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase">
            source → target
          </p>
          <div
            className="flex flex-col gap-2.5"
            role="listbox"
            aria-label="Translation pairs"
            aria-multiselectable="true"
          >
            {pairs.map((pair, index) => (
              <TranslationPairRow
                key={pair.id}
                leftId={`${pair.id}-source`}
                rightId={`${pair.id}-target`}
                leftValue={pair.source}
                rightValue={pair.target}
                leftPlaceholder="e.g. Mr. Smith"
                rightPlaceholder="e.g. Sr. Smith"
                leftAriaLabel={`Source ${index + 1}`}
                rightAriaLabel={`Target ${index + 1}`}
                selected={selectedPairs.has(pair.id)}
                onSelect={(additive, keepIfSelected) =>
                  toggleSelect(
                    pair.id,
                    additive,
                    setSelectedPairs,
                    keepIfSelected,
                  )
                }
                onLeftChange={(v) => {
                  const next = pairs.map((p) =>
                    p.id === pair.id ? { ...p, source: v } : p,
                  );
                  setPairs(next);
                  emit({ pairs: next });
                }}
                onRightChange={(v) => {
                  const next = pairs.map((p) =>
                    p.id === pair.id ? { ...p, target: v } : p,
                  );
                  setPairs(next);
                  emit({ pairs: next });
                }}
              />
            ))}
          </div>
        </div>
      </SettingsGroup>
    </div>
  );
}
