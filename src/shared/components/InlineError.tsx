import type { ReactNode } from "react";
import { X } from "lucide-react";

import { Alert, AlertDescription } from "@/shared/ui/alert";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

interface Props {
  children: ReactNode;
  onDismiss?: () => void;
  className?: string;
}

export default function InlineError({ children, onDismiss, className }: Props) {
  return (
    <Alert variant="destructive" className={cn("relative pr-10", className)}>
      <AlertDescription>{children}</AlertDescription>
      {onDismiss ? (
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="absolute top-2 right-2 text-destructive hover:text-destructive"
          onClick={onDismiss}
          aria-label="Dismiss error"
        >
          <X className="size-4" />
        </Button>
      ) : null}
    </Alert>
  );
}
