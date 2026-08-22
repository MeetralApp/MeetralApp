import {
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { ChevronDown } from "lucide-react";

import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";
import { GENERAL_KEY_PRESETS } from "@/shared/lib/contextPayload";

/** Input + suggestion list: type a custom key or pick a preset. */
export default function KeyCombobox({
  id,
  value,
  onChange,
  ariaLabel,
}: {
  id: string;
  value: string;
  onChange: (v: string) => void;
  ariaLabel: string;
}) {
  const listId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);

  const query = value.trim().toLowerCase();
  const filtered = GENERAL_KEY_PRESETS.filter((p) => {
    if (!query) return true;
    return (
      p.value.includes(query) || p.label.toLowerCase().includes(query)
    );
  });

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) {
        setOpen(false);
        setActiveIndex(-1);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  useEffect(() => {
    setActiveIndex(-1);
  }, [value, open]);

  const pick = (next: string) => {
    onChange(next);
    setOpen(false);
    setActiveIndex(-1);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setOpen(true);
      if (filtered.length === 0) return;
      setActiveIndex((i) => (i + 1) % filtered.length);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setOpen(true);
      if (filtered.length === 0) return;
      setActiveIndex((i) => (i <= 0 ? filtered.length - 1 : i - 1));
      return;
    }
    if (e.key === "Enter" && open && activeIndex >= 0 && filtered[activeIndex]) {
      e.preventDefault();
      pick(filtered[activeIndex].value);
      return;
    }
    if (e.key === "Escape") {
      setOpen(false);
      setActiveIndex(-1);
    }
  };

  return (
    <div ref={rootRef} className="relative min-w-0 w-full">
      <div className="relative">
        <Input
          id={id}
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={onKeyDown}
          placeholder="Key — type or pick"
          aria-label={ariaLabel}
          role="combobox"
          aria-expanded={open}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={
            activeIndex >= 0 && filtered[activeIndex]
              ? `${listId}-opt-${filtered[activeIndex].value}`
              : undefined
          }
          autoComplete="off"
          className="h-8 pr-8"
        />
        <button
          type="button"
          tabIndex={-1}
          className="absolute top-1/2 right-1 flex size-6 -translate-y-1/2 cursor-pointer items-center justify-center rounded-sm text-muted-foreground hover:text-foreground"
          aria-label="Show key suggestions"
          onClick={() => setOpen((o) => !o)}
        >
          <ChevronDown
            className={cn("size-3.5 transition-transform", open && "rotate-180")}
            aria-hidden
          />
        </button>
      </div>
      {open ? (
        <ul
          id={listId}
          role="listbox"
          data-key-suggestions=""
          className="absolute z-50 mt-1 max-h-48 w-full overflow-y-auto rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
        >
          {filtered.length > 0 ? (
            filtered.map((preset, index) => (
              <li key={preset.value} role="presentation">
                <button
                  type="button"
                  id={`${listId}-opt-${preset.value}`}
                  role="option"
                  aria-selected={value === preset.value}
                  className={cn(
                    "flex w-full cursor-pointer items-baseline gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none",
                    index === activeIndex
                      ? "bg-hover-surface text-foreground"
                      : "hover:bg-hover-surface",
                  )}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    pick(preset.value);
                  }}
                  onMouseEnter={() => setActiveIndex(index)}
                >
                  <span>{preset.label}</span>
                  <span className="font-mono text-[0.65rem] text-muted-foreground">
                    {preset.value}
                  </span>
                </button>
              </li>
            ))
          ) : (
            <li className="px-2 py-1.5 text-sm text-muted-foreground">
              {query
                ? `Use “${value.trim()}” as custom key`
                : "No suggestions"}
            </li>
          )}
        </ul>
      ) : null}
    </div>
  );
}
