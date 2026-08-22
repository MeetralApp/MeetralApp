export type SummaryBlockKind =
  | "overview"
  | "keyPoints"
  | "decisions"
  | "actionItems"
  | "openQuestions";

export interface SummarySegmentRef {
  direction: string;
  sequence: number;
}

export interface SummaryPoint {
  id: string;
  text: string;
  owner?: string | null;
  due?: string | null;
  segmentRefs: SummarySegmentRef[];
}

export interface SummaryPointBlock {
  points: SummaryPoint[];
}

/** LLM brief schema (Rust parse only — FE persists TipTap in generatedJson). */
export interface MeetingBriefSummary {
  schemaVersion?: number;
  overview: SummaryPointBlock;
  keyPoints: SummaryPointBlock;
  decisions?: SummaryPointBlock;
  actionItems?: SummaryPointBlock;
  openQuestions?: SummaryPointBlock;
}

export interface SummaryTemplateInfo {
  id: string;
  name: string;
  description: string;
}

export interface SummaryLanguageInfo {
  code: string;
  name: string;
}
