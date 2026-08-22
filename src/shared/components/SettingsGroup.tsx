import type { ReactNode } from "react";

import SectionHeading from "@/shared/components/SectionHeading";
import { cn } from "@/shared/lib/utils";

interface Props {
  title?: string;
  /** Optional `?` hint rendered immediately after the group title. */
  titleHint?: ReactNode;
  description?: string;
  /** Trailing control on the title row (e.g. API key chip). */
  headerEnd?: ReactNode;
  children: ReactNode;
  className?: string;
}

export default function SettingsGroup({
  title,
  titleHint,
  description,
  headerEnd,
  children,
  className,
}: Props) {
  const hasHeader =
    Boolean(title) ||
    Boolean(titleHint) ||
    Boolean(description) ||
    Boolean(headerEnd);

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {hasHeader ? (
        <div className="flex min-w-0 items-end justify-between gap-2">
          <div className="flex min-w-0 flex-col justify-end gap-0.5">
            {title || titleHint ? (
              <span className="inline-flex min-w-0 items-center gap-1.5 leading-none">
                {title ? <SectionHeading as="h4">{title}</SectionHeading> : null}
                {titleHint}
              </span>
            ) : null}
            {description ? (
              <p className="m-0 text-xs leading-relaxed text-muted-foreground">
                {description}
              </p>
            ) : null}
          </div>
          {headerEnd ? (
            <span className="inline-flex shrink-0 items-center">{headerEnd}</span>
          ) : null}
        </div>
      ) : null}
      {children}
    </div>
  );
}
