import { useCallback, useState, type KeyboardEvent } from "react";
import { X } from "lucide-react";

import { Badge } from "@/shared/ui/badge";
import { Input } from "@/shared/ui/input";

/** Tag input for many short recognition terms. */
export default function TermTagsInput({
  id,
  terms,
  onChange,
}: {
  id: string;
  terms: string[];
  onChange: (next: string[]) => void;
}) {
  const [draft, setDraft] = useState("");

  const addTerms = useCallback(
    (raw: string) => {
      const parts = raw
        .split(/[,;\n]/)
        .map((t) => t.trim())
        .filter(Boolean);
      if (parts.length === 0) return;
      const next = [...terms];
      for (const part of parts) {
        if (!next.some((t) => t.toLowerCase() === part.toLowerCase())) {
          next.push(part);
        }
      }
      onChange(next);
      setDraft("");
    },
    [onChange, terms],
  );

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addTerms(draft);
    } else if (e.key === "Backspace" && !draft && terms.length > 0) {
      onChange(terms.slice(0, -1));
    }
  };

  return (
    <div
      className="overflow-hidden rounded-md border border-input bg-transparent"
      role="group"
      aria-label="Recognition terms"
    >
      {terms.length > 0 ? (
        <div className="flex flex-wrap items-center gap-1.5 px-2.5 pt-2 pb-1.5">
          {terms.map((term) => (
            <Badge
              key={term}
              variant="secondary"
              className="h-6 gap-1 px-2 pr-1 font-normal"
            >
              {term}
              <button
                type="button"
                className="cursor-pointer rounded-sm p-0.5 hover:bg-muted"
                aria-label={`Remove term ${term}`}
                onClick={() => onChange(terms.filter((t) => t !== term))}
              >
                <X className="size-3" aria-hidden />
              </button>
            </Badge>
          ))}
        </div>
      ) : null}
      <Input
        id={id}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={onKeyDown}
        onBlur={() => {
          if (draft.trim()) addTerms(draft);
        }}
        placeholder={
          terms.length > 0 ? "Add term" : "Type a term, then Enter"
        }
        aria-label="Add recognition term"
        className={
          terms.length > 0
            ? "h-9 w-full rounded-none border-0 border-t border-input bg-transparent px-2.5 shadow-none focus-visible:ring-0"
            : "h-9 w-full border-0 bg-transparent px-2.5 shadow-none focus-visible:ring-0"
        }
      />
    </div>
  );
}
