import { cn } from "@/shared/lib/utils";

/**
* Shared trigger chrome for header selects / meeting menu.
* Matches `SelectTrigger` size=sm: rounded-md, truncate-friendly.
*/
export function headerChipTriggerClass(extra?: string): string {
  return cn(
    "h-8 w-full min-w-0 cursor-pointer overflow-hidden rounded-md border border-border bg-card px-2.5 text-xs shadow-none",
    "transition-[color,box-shadow] outline-none",
    "hover:bg-hover-surface",
    "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
    "disabled:cursor-not-allowed disabled:opacity-50",
    "*:data-[slot=select-value]:min-w-0 *:data-[slot=select-value]:flex-1 *:data-[slot=select-value]:truncate",
    extra,
  );
}
