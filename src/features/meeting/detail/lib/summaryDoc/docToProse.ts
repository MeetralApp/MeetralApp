import type { JSONContent } from "@tiptap/core";

import { citationLabel } from "./citationExtension";

/** TipTap JSON → export prose; citation atoms become `3:48·You`. */
export function docToProse(doc: JSONContent | null | undefined): string {
  if (!doc) return "";
  const lines: string[] = [];
  walkBlock(doc, lines, "");
  return lines.join("\n").trim();
}

function walkBlock(node: JSONContent, lines: string[], prefix: string) {
  const type = node.type ?? "";
  if (type === "doc") {
    for (const child of node.content ?? []) walkBlock(child, lines, prefix);
    return;
  }
  if (type === "heading") {
    const text = inlineToText(node).trim();
    if (text) lines.push(`${"#".repeat(Number(node.attrs?.level ?? 3))} ${text}`);
    lines.push("");
    return;
  }
  if (type === "paragraph") {
    const text = inlineToText(node).trim();
    if (text) lines.push(`${prefix}${text}`);
    return;
  }
  if (type === "bulletList" || type === "orderedList") {
    for (const item of node.content ?? []) {
      walkListItem(item, lines, type === "orderedList");
    }
    lines.push("");
    return;
  }
  if (type === "blockquote") {
    for (const child of node.content ?? []) walkBlock(child, lines, "> ");
    return;
  }
  for (const child of node.content ?? []) walkBlock(child, lines, prefix);
}

function walkListItem(node: JSONContent, lines: string[], ordered: boolean) {
  const marker = ordered ? "1. " : "- ";
  const children = node.content ?? [];
  if (children.length === 0) {
    lines.push(`${marker}`);
    return;
  }
  children.forEach((child, i) => {
    if (child.type === "paragraph") {
      const text = inlineToText(child).trim();
      lines.push(i === 0 ? `${marker}${text}` : `  ${text}`);
    } else if (child.type === "bulletList" || child.type === "orderedList") {
      walkBlock(child, lines, "  ");
    } else {
      walkBlock(child, lines, "  ");
    }
  });
}

function inlineToText(node: JSONContent): string {
  if (node.type === "text") return node.text ?? "";
  if (node.type === "hardBreak") return "\n";
  if (node.type === "citation") {
    return citationLabel({
      direction: String(node.attrs?.direction ?? "outbound"),
      startedAtMs: Number(node.attrs?.startedAtMs ?? 0),
    });
  }
  return (node.content ?? []).map(inlineToText).join("");
}
