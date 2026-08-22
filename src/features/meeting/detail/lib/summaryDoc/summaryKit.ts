import StarterKit from "@tiptap/starter-kit";

import { cn } from "@/shared/lib/utils";

/** Heading L3 — matches generate section labels. */
export const SUMMARY_HEADING_CLASS = cn(
  "mb-2 mt-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground first:mt-0",
);

export const SUMMARY_BULLET_LIST_CLASS = cn("my-1.5 list-none p-0");

/** Accent left border — matches generate point rows. */
export const SUMMARY_LIST_ITEM_CLASS = cn(
  "my-1.5 border-l-2 border-accent/30 py-0.5 pl-3 [&+li]:mt-2 [&>p]:m-0",
);

export const SUMMARY_PARAGRAPH_CLASS = cn("my-1");

export const SUMMARY_BOLD_CLASS = cn("font-semibold");

/**
* ProseMirror surface chrome (spacing + placeholder).
* Structural look lives on SummaryKit node HTMLAttributes.
*/
export const summarySurfaceClass = cn(
  "max-w-none text-sm leading-relaxed text-foreground focus:outline-none",
  "[&>*+*]:mt-2",
  "[&_.is-empty::before]:pointer-events-none [&_.is-empty::before]:float-left [&_.is-empty::before]:h-0 [&_.is-empty::before]:text-muted-foreground [&_.is-empty::before]:opacity-70 [&_.is-empty::before]:content-[attr(data-placeholder)]",
);

/**
* Narrow TipTap kit = generate vocabulary only:
* heading(3), bulletList, paragraph, bold, hardBreak, history.
* Citation is composed separately by the editor.
*/
export function summaryStarterKit() {
  return StarterKit.configure({
    heading: {
      levels: [3],
      HTMLAttributes: { class: SUMMARY_HEADING_CLASS },
    },
    bulletList: {
      HTMLAttributes: { class: SUMMARY_BULLET_LIST_CLASS },
    },
    listItem: {
      HTMLAttributes: { class: SUMMARY_LIST_ITEM_CLASS },
    },
    paragraph: {
      HTMLAttributes: { class: SUMMARY_PARAGRAPH_CLASS },
    },
    bold: {
      HTMLAttributes: { class: SUMMARY_BOLD_CLASS },
    },
    blockquote: false,
    code: false,
    codeBlock: false,
    italic: false,
    strike: false,
    orderedList: false,
    horizontalRule: false,
    link: false,
    underline: false,
  });
}
