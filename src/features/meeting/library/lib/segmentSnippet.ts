/** Cap for snippets stored in citation attrs / compact picker rows. */
export const ANCHOR_SNIPPET_CAP = 72;

/**
* Snippet text for anchor-badge tooltips (design-system/MASTER.md): prefers the
* translated text (what the user reads), whitespace-collapsed to one line.
* `maxLen` truncates for compact contexts (doc-atom attrs, picker rows).
*/
export function segmentSnippetText(
  segment: { sourceText: string; translatedText: string },
  maxLen?: number,
): string {
  const text = (segment.translatedText || segment.sourceText || "")
    .replace(/\s+/g, " ")
    .trim();
  return maxLen ? text.slice(0, maxLen) : text;
}
