import { useEffect, useMemo, useRef } from "react";
import { Bold } from "lucide-react";
import { EditorContent, useEditor, useEditorState } from "@tiptap/react";
import { BubbleMenu } from "@tiptap/react/menus";
import Placeholder from "@tiptap/extension-placeholder";
import type { JSONContent } from "@tiptap/core";
import type { Editor } from "@tiptap/react";

import { Citation, type CitationAttrs } from "../lib/summaryDoc/citationExtension";
import { CitationSuggestion } from "../lib/summaryDoc/citationSuggestion";
import {
  summaryStarterKit,
  summarySurfaceClass,
} from "../lib/summaryDoc/summaryKit";
import { StructureSuggestion } from "../lib/summaryDoc/structureSuggestion";
import { withCitationSnippets } from "../lib/summaryDoc/withCitationSnippets";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

export type SummaryDocEditorHandle = {
  getJSON: () => JSONContent;
  insertCitation: (attrs: CitationAttrs) => void;
  focus: () => void;
};

interface Props {
  initialDoc: JSONContent;
  segments: TranscriptSegment[];
  editable?: boolean;
  /** Fill parent and scroll inside the editor (edit mode). */
  fillHeight?: boolean;
  onCitationClick?: (attrs: CitationAttrs) => void;
  onReady?: (handle: SummaryDocEditorHandle) => void;
  className?: string;
}

function toHandle(editor: Editor): SummaryDocEditorHandle {
  return {
    getJSON: () => editor.getJSON(),
    insertCitation: (attrs) => {
      editor.chain().focus().insertCitation(attrs).run();
    },
    focus: () => {
      editor.chain().focus().run();
    },
  };
}

function BoldBubble({ editor }: { editor: Editor }) {
  const isBold = useEditorState({
    editor,
    selector: ({ editor: ed }) => ed.isActive("bold"),
  });

  return (
    <BubbleMenu
      editor={editor}
      options={{ placement: "top", offset: 8 }}
      shouldShow={({ editor: ed, from, to }) =>
        ed.isEditable && from !== to && ed.state.selection.empty === false
      }
    >
      <div className="flex items-center gap-0.5 rounded-md border border-border bg-popover p-0.5 shadow-md">
        <Button
          type="button"
          variant={isBold ? "secondary" : "ghost"}
          size="icon-xs"
          aria-label="Bold"
          aria-pressed={isBold}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => editor.chain().focus().toggleBold().run()}
        >
          <Bold aria-hidden strokeWidth={2.25} />
        </Button>
      </div>
    </BubbleMenu>
  );
}

export default function SummaryDocEditor({
  initialDoc,
  segments,
  editable = true,
  fillHeight = false,
  onCitationClick,
  onReady,
  className,
}: Props) {
  const segmentsRef = useRef(segments);
  segmentsRef.current = segments;
  const onCitationClickRef = useRef(onCitationClick);
  onCitationClickRef.current = onCitationClick;

  const extensions = useMemo(
    () => [
      summaryStarterKit(),
      Placeholder.configure({
        placeholder: editable
          ? "Edit summary… Type / for section or list · @ to cite."
          : "",
      }),
      Citation.configure({
        onCitationClick: (attrs) => onCitationClickRef.current?.(attrs),
      }),
      ...(editable
        ? [
            StructureSuggestion,
            CitationSuggestion.configure({
              getSegments: () => segmentsRef.current,
            }),
          ]
        : []),
    ],
    [editable],
  );

  // Enrich snippet-less citation atoms (Rust-seeded / legacy docs) from the
  // loaded transcript so tooltips match artifact chips (design-system/MASTER.md).
  const enrichedDoc = useMemo(
    () => withCitationSnippets(initialDoc, segments),
    [initialDoc, segments],
  );

  const editor = useEditor({
    extensions,
    content: enrichedDoc,
    editable,
    editorProps: {
      attributes: {
        class: cn(
          summarySurfaceClass,
          fillHeight || editable ? "min-h-[12rem]" : undefined,
          fillHeight && "min-h-full",
        ),
      },
    },
  });

  useEffect(() => {
    if (!editor || !onReady) return;
    onReady(toHandle(editor));
  }, [editor, onReady]);

  useEffect(() => {
    if (!editor) return;
    editor.setEditable(editable);
  }, [editor, editable]);

  if (!editor) return null;

  return (
    <div
      className={cn(
        fillHeight && "flex min-h-0 flex-1 flex-col overflow-hidden",
        editable
          ? "rounded-md border border-border/60 bg-secondary/25 focus-within:ring-1 focus-within:ring-ring/40"
          : "border-0 bg-transparent",
        className,
      )}
    >
      {editable ? <BoldBubble editor={editor} /> : null}
      <div
        className={cn(
          fillHeight && "min-h-0 flex-1 overflow-y-auto overscroll-contain",
        )}
      >
        <EditorContent
          editor={editor}
          className={cn(editable ? "px-3 py-2.5" : "px-0 py-0")}
        />
      </div>
    </div>
  );
}
