import { useState } from "react";
import { ListFilter } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import {
  TIMELINE_FILTER_OPTIONS,
  timelineFilterLabel,
  type TimelineDirectionFilter,
} from "../lib/timelineTranscript";

interface Props {
  filter: TimelineDirectionFilter;
  onFilterChange: (next: TimelineDirectionFilter) => void;
  /** Open menu upward over the floating player. */
  side?: "top" | "bottom";
}

/** Icon trigger → All / You / Meeting (timeline filter + audio source). */
export default function AudioSourceFilterMenu({
  filter,
  onFilterChange,
  side = "top",
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const label = timelineFilterLabel(filter);
  const triggerTip = `Source: ${label}`;

  return (
    <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
      <AppTooltip label={triggerTip} open={menuOpen ? false : undefined}>
        <span className="inline-flex">
          <DropdownMenuTrigger asChild>
            <Button
              type="button"
              variant="outline"
              size="icon-xs"
              aria-label={`Audio and transcript source: ${label}. Open to change.`}
            >
              <ListFilter aria-hidden />
            </Button>
          </DropdownMenuTrigger>
        </span>
      </AppTooltip>
      <DropdownMenuContent
        align="end"
        side={side}
        className="min-w-0 w-max p-0.5"
      >
        <DropdownMenuRadioGroup
          value={filter}
          onValueChange={(value) =>
            onFilterChange(value as TimelineDirectionFilter)
          }
        >
          {TIMELINE_FILTER_OPTIONS.map((opt) => (
            <AppTooltip key={opt.id} label={opt.title} side="left">
              <DropdownMenuRadioItem
                value={opt.id}
                className="py-1 pr-2.5 pl-6 text-xs"
              >
                {opt.label}
              </DropdownMenuRadioItem>
            </AppTooltip>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
