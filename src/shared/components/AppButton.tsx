import type { ComponentProps } from "react";

import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

export type AppButtonProps = ComponentProps<typeof Button> & {
  destructive?: boolean;
};

export function AppButton({
  destructive = false,
  variant,
  className,
  ...props
}: AppButtonProps) {
  const resolvedVariant =
    variant ??
    (destructive ? "outline" : props.size === "icon" ? "ghost" : "default");

  return (
    <Button
      variant={resolvedVariant}
      className={cn(
        destructive &&
          "border-destructive/45 text-destructive hover:bg-destructive/10 hover:text-destructive",
        className,
      )}
      {...props}
    />
  );
}

export { Button };
