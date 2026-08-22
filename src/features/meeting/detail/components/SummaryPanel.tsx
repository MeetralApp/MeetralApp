import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { FileText, Loader2, Pencil } from "lucide-react";
import type { JSONContent } from "@tiptap/core";

import SummaryDocEditor, {
  type SummaryDocEditorHandle,
} from "./SummaryDocEditor";
import SummaryDocView from "./SummaryDocView";
import SummaryGenerateChip from "./SummaryGenerateChip";
import EmptyState from "@/shared/components/EmptyState";
import InlineError from "@/shared/components/InlineError";
import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import { pipelineToolbarClasses } from "@/features/pipeline/lib/pipelineColors";
import { formatSummaryError } from "@/features/meeting/library/lib/meetingDisplay";
import { listSummaryTemplates } from "@/features/meeting/library/lib/meetingApi";
import { useAiCatalog } from "@/features/ai/hooks/useAiCatalog";
import { isNotesSession } from "@/features/pipeline/lib/sessionMode";
import { summaryDocHasContent } from "../lib/summaryDisplay";
import { DEFAULT_SUMMARY_TEMPLATE_ID } from "../lib/summaryConstants";
import type { CitationAttrs } from "../lib/summaryDoc/citationExtension";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import type {
  MeetingRecord,
  MeetingSummary,
  SummaryProgressUi,
  TranscriptSegment,
} from "@/features/meeting/library/lib/meetingTypes";
import {
  ANCHOR_SNIPPET_CAP,
  segmentSnippetText,
} from "@/features/meeting/library/lib/segmentSnippet";
import type {
  SummaryLanguageInfo,
  SummaryTemplateInfo,
} from "../lib/summaryTypes";

interface Props {
  meeting: MeetingRecord;
  aiProvider: AiProvider;
  segments: TranscriptSegment[];
  summary: MeetingSummary | null;
  summaryLoading: boolean;
  summaryProgress?: SummaryProgressUi | null;
  summaryError: string | null;
  /** Intelligence answer language; "" = match the meeting's You language. */
  answerLanguage?: string;
  onGenerate: (templateId: string, language: string) => void;
  onSaveEdit: (docJson: string) => Promise<void>;
  onDismissError: () => void;
  onCitationClick: (attrs: CitationAttrs) => void;
  onReturnToLive?: () => void;
  onDocEditingChange?: (editing: boolean) => void;
  registerCiteInserter?: (
    inserter: ((segment: TranscriptSegment) => void) | null,
  ) => void;
  /** Artifacts (or other) chrome under the summary scroll body. */
  footer?: ReactNode;
}

function parseDocJson(raw: string): JSONContent | null {
  try {
    const doc = JSON.parse(raw) as JSONContent;
    if (doc?.type !== "doc") return null;
    return doc;
  } catch {
    return null;
  }
}

export default function SummaryPanel({
  meeting,
  aiProvider,
  segments,
  summary,
  summaryLoading,
  summaryProgress,
  summaryError,
  answerLanguage = "",
  onGenerate,
  onSaveEdit,
  onDismissError,
  onCitationClick,
  onReturnToLive,
  onDocEditingChange,
  registerCiteInserter,
  footer,
}: Props) {
  const catalog = useAiCatalog(aiProvider);
  const [templates, setTemplates] = useState<SummaryTemplateInfo[]>([]);
  const [templateId, setTemplateId] = useState(DEFAULT_SUMMARY_TEMPLATE_ID);
  const [language, setLanguage] = useState(
    () => answerLanguage.trim() || meeting.meetingLanguage,
  );
  const [editing, setEditing] = useState(false);
  const [editorDoc, setEditorDoc] = useState<JSONContent | null>(null);
  const [saving, setSaving] = useState(false);
  const [generateOpen, setGenerateOpen] = useState(false);
  const editorHandleRef = useRef<SummaryDocEditorHandle | null>(null);

  const languages = useMemo<SummaryLanguageInfo[]>(
    () =>
      catalog.languages.map((lang) => ({
        code: lang.code,
        name: lang.name,
      })),
    [catalog.languages],
  );

  const hasTranscript = segments.length > 0;
  const notesMode = isNotesSession(meeting);
  const canGenerate = meeting.status !== "live" && hasTranscript;
  const friendlyError = summaryError ? formatSummaryError(summaryError) : null;
  const showGenerateDock = hasTranscript || meeting.status === "live";

  const doc = useMemo(
    () => (summary ? parseDocJson(summary.generatedJson) : null),
    [summary],
  );
  const hasSummary = Boolean(
    summary &&
      (summaryDocHasContent(summary.generatedJson) ||
        summary.generatedText.trim()),
  );

  useEffect(() => {
    void listSummaryTemplates().then(setTemplates);
  }, []);

  useEffect(() => {
    if (summary?.summaryLanguage) {
      setLanguage(summary.summaryLanguage);
    } else if (answerLanguage.trim()) {
      setLanguage(answerLanguage.trim());
    } else {
      setLanguage(meeting.meetingLanguage);
    }
    if (summary?.templateId) {
      setTemplateId(summary.templateId);
    }
  }, [
    summary?.summaryLanguage,
    summary?.templateId,
    meeting.meetingLanguage,
    meeting.id,
    answerLanguage,
  ]);

  useEffect(() => {
    setEditing(false);
    setEditorDoc(null);
    editorHandleRef.current = null;
    setGenerateOpen(false);
  }, [meeting.id, summary?.generatedAtMs]);

  useEffect(() => {
    onDocEditingChange?.(editing);
  }, [editing, onDocEditingChange]);

  const insertCiteFromSegment = useCallback((segment: TranscriptSegment) => {
    editorHandleRef.current?.insertCitation({
      segmentId: segment.id,
      direction: segment.direction,
      startedAtMs: segment.startedAtMs,
      snippet: segmentSnippetText(segment, ANCHOR_SNIPPET_CAP),
    });
  }, []);

  useEffect(() => {
    if (!registerCiteInserter) return;
    registerCiteInserter(editing ? insertCiteFromSegment : null);
    return () => registerCiteInserter(null);
  }, [editing, insertCiteFromSegment, registerCiteInserter]);

  const beginEdit = useCallback(() => {
    if (!summary || !doc) return;
    setGenerateOpen(false);
    setEditorDoc(doc);
    setEditing(true);
  }, [summary, doc]);

  const cancelEdit = useCallback(() => {
    setEditing(false);
    setEditorDoc(null);
    editorHandleRef.current = null;
  }, []);

  const saveEdit = useCallback(async () => {
    const handle = editorHandleRef.current;
    if (!handle) return;
    setSaving(true);
    try {
      const json = handle.getJSON();
      await onSaveEdit(JSON.stringify(json));
      setEditing(false);
      setEditorDoc(null);
      editorHandleRef.current = null;
    } finally {
      setSaving(false);
    }
  }, [onSaveEdit]);

  const requestGenerate = () => {
    onGenerate(templateId, language);
  };

  const handleCitationClick = useCallback(
    (attrs: CitationAttrs) => {
      if (!attrs.segmentId) return;
      onCitationClick(attrs);
    },
    [onCitationClick],
  );

  const generateProps = {
    templates,
    languages,
    templateId,
    language,
    canGenerate,
    summaryLoading,
    summaryProgress,
    hasSummary,
    editing,
    meetingStatus: meeting.status,
    hasTranscript,
    notesMode,
    open: generateOpen,
    onOpenChange: setGenerateOpen,
    onTemplateChange: setTemplateId,
    onLanguageChange: setLanguage,
    onGenerate: requestGenerate,
  };

  let body: ReactNode;
  if (!hasTranscript && meeting.status !== "live") {
    body = (
      <EmptyState
        icon={FileText}
        title={notesMode ? "No notes to summarize" : "No transcript to summarize"}
        description={
          notesMode
            ? "Capture notes in a live Notes session first. AI summarizes after you end the meeting."
            : "Record a meeting first. AI summarizes both You and Meeting column transcripts after you end the session."
        }
        action={
          onReturnToLive ? (
            <Button type="button" size="sm" onClick={onReturnToLive}>
              Start new meeting
            </Button>
          ) : undefined
        }
      />
    );
  } else if (!hasSummary || !summary || !doc) {
    body = (
      <p className="m-0 text-sm text-muted-foreground">
        {notesMode
          ? "No meeting notes summary yet. Use Summary options below to pick a template and language, then generate."
          : "No summary yet. Use Summary options below to pick a template and language, then generate."}
      </p>
    );
  } else if (editing && editorDoc) {
    body = (
      <div className="flex min-h-0 flex-1 flex-col gap-2">
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <SummaryDocEditor
            key={`${meeting.id}-${summary.generatedAtMs ?? 0}-edit`}
            initialDoc={editorDoc}
            segments={segments}
            editable
            fillHeight
            onCitationClick={handleCitationClick}
            onReady={(handle) => {
              editorHandleRef.current = handle;
            }}
          />
        </div>
        <div className="flex shrink-0 items-center gap-2 border-t border-border/60 pt-2">
          <p className="m-0 min-w-0 flex-1 text-xs text-muted-foreground">
            Type / for section or list · click a transcript line or type @ to
            cite.
          </p>
          <Button
            type="button"
            size="sm"
            className="h-7 text-xs"
            onClick={() => void saveEdit()}
            disabled={saving}
          >
            {saving ? <Loader2 className="size-3.5 animate-spin" /> : "Save"}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 text-xs"
            onClick={cancelEdit}
            disabled={saving}
          >
            Cancel
          </Button>
        </div>
      </div>
    );
  } else {
    body = (
      <div className="relative">
        {/* Sticky edit chip only (no full-width wash — that read as a bar). */}
        <div className="sticky top-0 z-[2] float-right mb-1 ml-2">
          <AppTooltip label="Edit summary">
            <Button
              type="button"
              variant="outline"
              size="icon-sm"
              aria-label="Edit summary"
              disabled={summaryLoading}
              className={pipelineToolbarClasses.paneToolbarActionChip}
              onClick={beginEdit}
            >
              <Pencil aria-hidden strokeWidth={2} />
            </Button>
          </AppTooltip>
        </div>
        <SummaryDocView
          docJson={summary.generatedJson}
          segments={segments}
          onCitationClick={handleCitationClick}
        />
      </div>
    );
  }

  return (
    <div className="relative flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden bg-card">
        {friendlyError && (
          <InlineError className="mx-4 mt-2 shrink-0" onDismiss={onDismissError}>
            {friendlyError}
          </InlineError>
        )}
        <div
          className={cn(
            "min-h-0 flex-1 px-4 py-3",
            editing ? "flex flex-col overflow-hidden" : "overflow-y-auto",
          )}
        >
          {body}
        </div>
        {footer}
      </div>

      {showGenerateDock ? (
        <div
          className={cn(
            "pointer-events-none absolute inset-x-0 bottom-0 z-[20]",
            "flex flex-col items-end justify-end gap-2 p-3",
          )}
        >
          <div className="pointer-events-auto">
            <SummaryGenerateChip {...generateProps} />
          </div>
        </div>
      ) : null}
    </div>
  );
}
