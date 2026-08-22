import { useEffect, useRef } from "react";

import { Button } from "@/shared/ui/button";
import { Textarea } from "@/shared/ui/textarea";
import { cn } from "@/shared/lib/utils";

/** Cap auto-grow so the dropdown/menu does not take over the viewport. */
const TEXTAREA_MAX_HEIGHT_PX = 128;

interface Props {
  value?: string;
  placeholder?: string;
  submitLabel?: string;
  maxLength?: number;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}

export default function InlineTextForm({
  value = "",
  placeholder = "Name",
  submitLabel = "Save",
  maxLength,
  onSubmit,
  onCancel,
}: Props) {
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const syncHeight = () => {
    const el = inputRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, TEXTAREA_MAX_HEIGHT_PX)}px`;
  };

  useEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    el.focus();
    el.select();
    syncHeight();
  }, []);

  const submit = () => {
    const next = inputRef.current?.value.trim() ?? "";
    if (!next) {
      onCancel();
      return;
    }
    onSubmit(next);
  };

  return (
    <form
      className="flex flex-col gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <Textarea
        ref={inputRef}
        defaultValue={value}
        placeholder={placeholder}
        aria-label={placeholder}
        maxLength={maxLength}
        rows={1}
        className={cn(
          "min-h-9 max-h-32 resize-none overflow-y-auto py-2 text-sm leading-snug",
          "field-sizing-content md:text-sm",
        )}
        onInput={syncHeight}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
            return;
          }
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            submit();
          }
        }}
      />
      <div className="flex justify-end gap-1.5">
        <Button type="button" variant="secondary" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" size="sm">
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}
