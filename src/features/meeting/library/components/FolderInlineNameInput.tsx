import { useEffect, useRef } from "react";

import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";

interface Props {
  value?: string;
  placeholder?: string;
  maxLength?: number;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}

export default function FolderInlineNameInput({
  value = "",
  placeholder = "Folder name",
  maxLength = 80,
  onSubmit,
  onCancel,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    if (value) inputRef.current?.select();
  }, [value]);

  const commit = () => {
    const next = inputRef.current?.value.trim() ?? "";
    if (!next) {
      onCancel();
      return;
    }
    onSubmit(next);
  };

  return (
    <Input
      ref={inputRef}
      type="text"
      defaultValue={value}
      placeholder={placeholder}
      aria-label={placeholder}
      maxLength={maxLength}
      className={cn("h-7 min-w-0 flex-1 px-1 py-0 text-sm")}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          commit();
        }
        if (e.key === "Escape") {
          e.preventDefault();
          onCancel();
        }
      }}
      onBlur={() => commit()}
    />
  );
}
