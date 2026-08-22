import type { JSONContent } from "@tiptap/core";

/** Unique citation segmentIds in document order. */
export function extractCiteSegmentIds(
  doc: JSONContent | null | undefined,
): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  walk(doc, out, seen);
  return out;
}

function walk(
  node: JSONContent | null | undefined,
  out: string[],
  seen: Set<string>,
) {
  if (!node) return;
  if (node.type === "citation") {
    const id = String(node.attrs?.segmentId ?? "").trim();
    if (id && !seen.has(id)) {
      seen.add(id);
      out.push(id);
    }
  }
  for (const child of node.content ?? []) {
    walk(child, out, seen);
  }
}
