import { forwardRef } from "react";
import { History } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

interface Props {
  onClick: () => void;
  active?: boolean;
}

const HistoryButton = forwardRef<HTMLButtonElement, Props>(
  function HistoryButton({ onClick, active }, ref) {
    return (
      <AppTooltip label="Meeting history">
        <Button
          ref={ref}
          type="button"
          variant="ghost"
          size="icon"
          className={cn(active && "bg-secondary")}
          onClick={onClick}
          aria-label="Meeting history"
          aria-expanded={active}
          aria-haspopup="dialog"
        >
          <History size={20} aria-hidden strokeWidth={2} />
        </Button>
      </AppTooltip>
    );
  },
);

export default HistoryButton;
