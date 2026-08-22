import type { JSONContent } from "@tiptap/core";

import type {
  SummaryBlockKind,
  SummaryPoint,
  MeetingBriefSummary,
} from "./summaryTypes";

export type {
  SummaryBlockKind,
  SummaryPoint,
  MeetingBriefSummary,
  SummaryLanguageInfo,
  SummaryTemplateInfo,
} from "./summaryTypes";

export function blockLabel(
  blockKind: SummaryBlockKind,
  language: string,
): string {
  const lang = language.toLowerCase();
  if (lang === "vi") {
    switch (blockKind) {
      case "overview":
        return "Tổng quan";
      case "keyPoints":
        return "Những điểm quan trọng";
      case "decisions":
        return "Quyết định";
      case "actionItems":
        return "Việc cần làm";
      case "openQuestions":
        return "Câu hỏi còn mở";
    }
  }
  switch (blockKind) {
    case "overview":
      return "Overview";
    case "keyPoints":
      return "Key points";
    case "decisions":
      return "Decisions";
    case "actionItems":
      return "Action items";
    case "openQuestions":
      return "Open questions";
  }
}

export function anchorColumnLabel(direction: string): string {
  return direction === "outbound" ? "You" : "Meeting";
}

export const SUMMARY_BLOCKS: SummaryBlockKind[] = [
  "overview",
  "keyPoints",
  "decisions",
  "actionItems",
  "openQuestions",
];

export function blockPoints(
  summary: MeetingBriefSummary,
  blockKind: SummaryBlockKind,
): SummaryPoint[] {
  switch (blockKind) {
    case "overview":
      return summary.overview?.points ?? [];
    case "keyPoints":
      return summary.keyPoints?.points ?? [];
    case "decisions":
      return summary.decisions?.points ?? [];
    case "actionItems":
      return summary.actionItems?.points ?? [];
    case "openQuestions":
      return summary.openQuestions?.points ?? [];
  }
}

/** True when `generated_json` is a TipTap doc with at least one block. */
export function summaryDocHasContent(docJson: string): boolean {
  try {
    const doc = JSON.parse(docJson) as JSONContent;
    if (doc?.type !== "doc") return false;
    return Array.isArray(doc.content) && doc.content.length > 0;
  } catch {
    return false;
  }
}
