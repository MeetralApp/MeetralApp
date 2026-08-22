import type { ReactNode } from "react";

import SectionHeading from "@/shared/components/SectionHeading";
import SettingInfoHint from "@/shared/components/SettingInfoHint";

export default function SectionHeader({
  title,
  hintLabel,
  hint,
  action,
}: {
  title: string;
  hintLabel: string;
  hint: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="inline-flex items-center gap-1.5">
        <SectionHeading as="h4">{title}</SectionHeading>
        <SettingInfoHint label={hintLabel}>{hint}</SettingInfoHint>
      </span>
      {action}
    </div>
  );
}
