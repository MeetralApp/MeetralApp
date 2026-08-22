import { Extension } from "@tiptap/core";
import Suggestion from "@tiptap/suggestion";
import { PluginKey } from "@tiptap/pm/state";
import type { Editor, Range } from "@tiptap/core";

import { createSuggestionPopupRenderer } from "./suggestionPopup";

export type StructureSuggestionItem = {
  id: "section" | "bullet";
  title: string;
  description: string;
  keywords: string[];
  run: (editor: Editor, range: Range) => void;
};

export const STRUCTURE_SUGGESTION_ITEMS: StructureSuggestionItem[] = [
  {
    id: "section",
    title: "Section",
    description: "Section heading (like generated blocks)",
    keywords: ["section", "heading", "h3", "title"],
    run: (editor, range) => {
      editor.chain().focus().deleteRange(range).setHeading({ level: 3 }).run();
    },
  },
  {
    id: "bullet",
    title: "Bullet list",
    description: "List of points",
    keywords: ["bullet", "list", "ul", "points"],
    run: (editor, range) => {
      editor.chain().focus().deleteRange(range).toggleBulletList().run();
    },
  },
];

export function filterStructureItems(query: string): StructureSuggestionItem[] {
  const q = query.trim().toLowerCase();
  if (!q) return STRUCTURE_SUGGESTION_ITEMS;
  return STRUCTURE_SUGGESTION_ITEMS.filter(
    (item) =>
      item.title.toLowerCase().includes(q) ||
      item.description.toLowerCase().includes(q) ||
      item.keywords.some((k) => k.includes(q)),
  );
}

/** Suggestion on `/` → Section heading or Bullet list (generate vocabulary). */
export const StructureSuggestion = Extension.create({
  name: "structureSuggestion",

  addProseMirrorPlugins() {
    return [
      Suggestion<StructureSuggestionItem, StructureSuggestionItem>({
        editor: this.editor,
        pluginKey: new PluginKey("structureSuggestionSlash"),
        char: "/",
        allowSpaces: false,
        startOfLine: false,
        items: ({ query }) => filterStructureItems(query),
        command: ({ editor, range, props }) => {
          props.run(editor, range);
        },
        render: createSuggestionPopupRenderer<StructureSuggestionItem>({
          emptyLabel: "No matching commands",
          widthClass: "w-64",
          getRow: (item) => ({
            title: item.title,
            subtitle: item.description,
          }),
        }),
      }),
    ];
  },
});
