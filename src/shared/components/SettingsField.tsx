import type { ReactNode } from "react";

import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";

interface Props {
  /** Field label text. Omit when using a custom `label` node. */
  label?: string;
  /** Custom label node (e.g. label + hint). Overrides `label` string. */
  labelContent?: ReactNode;
  htmlFor?: string;
  /** Trailing secondary actions (Refresh, Preview). */
  actions?: ReactNode;
  /** L4 description under the control. */
  description?: ReactNode;
  children: ReactNode;
  className?: string;
}

/**
* L3 Settings field row: label (+ actions) → control → optional description.
* Use for every control in Settings so tabs share one layout.
*/
export default function SettingsField({
  label,
  labelContent,
  htmlFor,
  actions,
  description,
  children,
  className,
}: Props) {
  const labelNode =
    labelContent ??
    (label ? (
      <Label htmlFor={htmlFor} className={settingsFieldLabelClass}>
        {label}
      </Label>
    ) : null);

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      {labelNode || actions ? (
        <div className="flex min-w-0 items-end justify-between gap-2">
          <span className="inline-flex min-w-0 items-center gap-1.5 leading-none">
            {labelNode}
          </span>
          {actions ? (
            <span className="inline-flex shrink-0 items-center gap-0.5">
              {actions}
            </span>
          ) : null}
        </div>
      ) : null}
      {children}
      {description ? (
        <p className="m-0 text-xs leading-relaxed text-muted-foreground">
          {description}
        </p>
      ) : null}
    </div>
  );
}
