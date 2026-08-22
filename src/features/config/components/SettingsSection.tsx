import { useEffect, useRef, useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

import AppBadge from "@/shared/components/AppBadge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/shared/ui/collapsible";
import { cn } from "@/shared/lib/utils";

export interface SectionStatus {
  tone: "ok" | "warn" | "muted";
  label: string;
}

interface Props {
  id?: string;
  title?: string;
  /** Optional `?` hint rendered immediately after the section title. */
  titleHint?: ReactNode;
  /** Trailing header control (e.g. Engine API key chip), before status badge. */
  headerEnd?: ReactNode;
  status?: SectionStatus;
  collapsible?: boolean;
  defaultCollapsed?: boolean;
  focusKey?: string;
  activeFocus?: string | null;
  children: ReactNode;
}

export default function SettingsSection({
  id,
  title,
  titleHint,
  headerEnd,
  status,
  collapsible = false,
  defaultCollapsed = false,
  focusKey,
  activeFocus,
  children,
}: Props) {
  const ref = useRef<HTMLElement>(null);
  const [collapsed, setCollapsed] = useState(defaultCollapsed);
  const hasHeader =
    Boolean(title) || Boolean(status) || Boolean(headerEnd);

  useEffect(() => {
    if (activeFocus && focusKey === activeFocus && ref.current) {
      if (collapsible) setCollapsed(false);
      ref.current.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  }, [activeFocus, focusKey, collapsible]);

  const header = hasHeader ? (
    <div className="flex min-w-0 flex-1 items-end justify-between gap-2">
      <span className="inline-flex min-w-0 items-center gap-1.5 leading-none">
        {title ? (
          <h3 className="m-0 text-sm font-semibold tracking-tight text-foreground">
            {title}
          </h3>
        ) : null}
        {titleHint}
      </span>
      {headerEnd || status ? (
        <span className="inline-flex shrink-0 items-center gap-0.5">
          {headerEnd}
          {status ? <AppBadge label={status.label} tone={status.tone} /> : null}
        </span>
      ) : null}
    </div>
  ) : null;

  if (collapsible) {
    return (
      <Collapsible
        open={!collapsed}
        onOpenChange={(open) => setCollapsed(!open)}
      >
        <section
          id={id}
          ref={ref}
          className="border-b border-border/40 py-4 last:border-b-0 last:pb-2"
        >
          <CollapsibleTrigger asChild>
            <button
              type="button"
              className="flex w-full cursor-pointer items-center justify-between gap-2 rounded-md text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
              aria-expanded={!collapsed}
            >
              {header}
              <ChevronDown
                className={cn(
                  "size-4 shrink-0 text-muted-foreground transition-transform",
                  !collapsed && "rotate-180",
                )}
                aria-hidden
              />
            </button>
          </CollapsibleTrigger>
          <CollapsibleContent className="pt-2">
            <div className="flex flex-col gap-4">{children}</div>
          </CollapsibleContent>
        </section>
      </Collapsible>
    );
  }

  return (
    <section
      id={id}
      ref={ref}
      className="border-b border-border/40 py-4 last:border-b-0 last:pb-2"
    >
      {header}
      <div className={cn("flex flex-col gap-4", header && "pt-2")}>{children}</div>
    </section>
  );
}
