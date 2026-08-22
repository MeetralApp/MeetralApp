import type { ComponentProps, ReactNode } from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/shared/ui/tooltip";
import { cn } from "@/shared/lib/utils";

type Side = NonNullable<ComponentProps<typeof TooltipContent>["side"]>;

type Props = {
  /** Tooltip body. Empty / null skips the wrapper (children only). */
  label: ReactNode;
  children: ReactNode;
  side?: Side;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** Extra classes on `TooltipContent` (e.g. wider max-width). */
  contentClassName?: string;
};

/**
* App-wide tooltip chrome — single style/flow for the design system.
* Content defaults live on `TooltipContent` (popover surface, border, fade-only).
*/
export default function AppTooltip({
  label,
  children,
  side = "bottom",
  open,
  onOpenChange,
  contentClassName,
}: Props) {
  if (label == null || label === false || label === "") {
    return <>{children}</>;
  }

  return (
    <Tooltip open={open} onOpenChange={onOpenChange}>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side} className={cn(contentClassName)}>
        {label}
      </TooltipContent>
    </Tooltip>
  );
}
