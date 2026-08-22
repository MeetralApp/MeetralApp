import type { ElementType, ReactNode } from "react";

import { cn } from "@/shared/lib/utils";

interface Props {
  as?: ElementType;
  children: ReactNode;
  className?: string;
}

export default function SectionHeading({
  as: Tag = "p",
  children,
  className,
}: Props) {
  return (
    <Tag
      className={cn(
        "m-0 text-xs font-semibold uppercase tracking-wider text-muted-foreground",
        className,
      )}
    >
      {children}
    </Tag>
  );
}
