import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/shared/lib/utils";

interface Props {
  icon?: LucideIcon;
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
}

export default function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: Props) {
  return (
    <div
      className={cn(
        "flex min-h-[200px] h-full flex-col items-center justify-center gap-3 p-6 text-center",
        className,
      )}
    >
      {Icon ? (
        <Icon className="size-10 text-muted-foreground/70" aria-hidden />
      ) : null}
      <div className="space-y-1">
        <p className="m-0 text-sm font-medium text-foreground">{title}</p>
        {description ? (
          <p className="m-0 text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {action}
    </div>
  );
}
