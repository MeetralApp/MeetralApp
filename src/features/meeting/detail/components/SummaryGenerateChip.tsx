import { Loader2, Sparkles } from "lucide-react";

import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/shared/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import { cn } from "@/shared/lib/utils";
import type { SummaryLanguageInfo, SummaryTemplateInfo } from "../lib/summaryTypes";
import type { SummaryProgressUi } from "@/features/meeting/library/lib/meetingTypes";

export interface SummaryGenerateProps {
  templates: SummaryTemplateInfo[];
  languages: SummaryLanguageInfo[];
  templateId: string;
  language: string;
  canGenerate: boolean;
  summaryLoading: boolean;
  summaryProgress?: SummaryProgressUi | null;
  hasSummary: boolean;
  editing: boolean;
  meetingStatus: string;
  hasTranscript: boolean;
  notesMode?: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onTemplateChange: (templateId: string) => void;
  onLanguageChange: (language: string) => void;
  onGenerate: () => void;
}

/**
* AI-dock Generate control: Sparkles trigger + compact upward Popover
* (click-outside dismisses; no sheet header).
*/
export default function SummaryGenerateChip({
  templates,
  languages,
  templateId,
  language,
  canGenerate,
  summaryLoading,
  summaryProgress,
  hasSummary,
  editing,
  meetingStatus,
  hasTranscript,
  notesMode = false,
  open,
  onOpenChange,
  onTemplateChange,
  onLanguageChange,
  onGenerate,
}: SummaryGenerateProps) {
  const triggerLabel = summaryLoading
    ? (summaryProgress?.detail ?? "Generating summary")
    : notesMode
      ? "Notes options"
      : "Summary options";

  const progressTooltip =
    summaryLoading && summaryProgress
      ? [summaryProgress.detail, summaryProgress.partialText]
          .filter(Boolean)
          .join("\n\n")
      : triggerLabel;

  const generateTitle =
    meetingStatus === "live"
      ? "End meeting first"
      : !hasTranscript
        ? notesMode
          ? "No notes to summarize"
          : "No transcript to summarize"
        : notesMode
          ? "Generate meeting notes with AI"
          : "Generate summary with AI";

  const buttonAriaLabel = summaryLoading
    ? (summaryProgress?.detail ?? "Generating summary")
    : hasSummary
      ? "Regenerate summary"
      : "Generate summary";

  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <AppTooltip label={progressTooltip} open={open ? false : undefined}>
        <span className="inline-flex">
          <PopoverTrigger asChild>
            <Button
              type="button"
              size="sm"
              variant={open ? "secondary" : "outline"}
              className={cn(
                "pointer-events-auto h-10 w-10 rounded-full p-0",
                "shadow-[var(--pane-elevated)] ring-1 ring-foreground/10",
                open && "ring-border/80 dark:ring-white/25",
              )}
              aria-label={triggerLabel}
              aria-expanded={open}
              aria-busy={summaryLoading || undefined}
            >
              {summaryLoading ? (
                <Loader2 className="size-4 animate-spin" aria-hidden />
              ) : (
                <Sparkles className="size-4" aria-hidden strokeWidth={2} />
              )}
            </Button>
          </PopoverTrigger>
        </span>
      </AppTooltip>

      <PopoverContent
        side="top"
        align="end"
        sideOffset={8}
        className="w-52 space-y-2.5 p-2.5"
        onOpenAutoFocus={(event) => event.preventDefault()}
        onInteractOutside={(event) => {
          // Select menus portal outside the popover — don't dismiss on pick.
          const target = event.target;
          if (
            target instanceof Element &&
            target.closest("[data-slot='select-content']")
          ) {
            event.preventDefault();
          }
        }}
      >
        <div className="space-y-1">
          <Label
            htmlFor="summary-template"
            className="text-[0.7rem] text-muted-foreground"
          >
            Template
          </Label>
          <Select value={templateId} onValueChange={onTemplateChange}>
            <SelectTrigger
              id="summary-template"
              size="sm"
              className="h-8 w-full min-w-0 text-xs"
              aria-label="Summary template"
            >
              <SelectValue placeholder="Template" />
            </SelectTrigger>
            <SelectContent position="popper" className="max-h-60">
              {templates.map((template) => (
                <SelectItem key={template.id} value={template.id}>
                  {template.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-1">
          <Label
            htmlFor="summary-language"
            className="text-[0.7rem] text-muted-foreground"
          >
            Language
          </Label>
          <Select value={language} onValueChange={onLanguageChange}>
            <SelectTrigger
              id="summary-language"
              size="sm"
              className="h-8 w-full min-w-0 text-xs"
              aria-label="Summary language"
            >
              <SelectValue placeholder="Language" />
            </SelectTrigger>
            <SelectContent position="popper" className="max-h-60">
              {languages.map((lang) => (
                <SelectItem key={lang.code} value={lang.code}>
                  {lang.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <AppTooltip label={generateTitle}>
          <Button
            type="button"
            size="sm"
            className="h-8 w-full text-xs"
            disabled={!canGenerate || summaryLoading || editing}
            onClick={() => {
              onGenerate();
              onOpenChange(false);
            }}
            aria-busy={summaryLoading || undefined}
            aria-label={buttonAriaLabel}
          >
            {summaryLoading ? (
              <>
                <Loader2 className="size-3.5 animate-spin" aria-hidden />
                <span className="max-w-[9rem] truncate">
                  {summaryProgress?.label ?? "Working…"}
                </span>
              </>
            ) : hasSummary ? (
              "Regenerate"
            ) : (
              "Generate"
            )}
          </Button>
        </AppTooltip>
      </PopoverContent>
    </Popover>
  );
}
