/** Escape a string for safe use inside `RegExp`. */
export function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Split query into unique tokens (≥2 chars preferred; keep 1-char if only token). */
export function queryTokens(query: string): string[] {
  const raw = query
    .trim()
    .split(/\s+/)
    .map((t) => t.trim())
    .filter(Boolean);
  if (raw.length === 0) return [];
  const filtered = raw.filter((t) => t.length >= 2);
  const tokens = filtered.length > 0 ? filtered : raw;
  const seen = new Set<string>();
  const out: string[] = [];
  for (const t of tokens) {
    const key = t.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(t);
  }
  return out;
}

export function textMatchesQuery(text: string, query: string): boolean {
  const tokens = queryTokens(query);
  if (tokens.length === 0 || !text) return false;
  const lower = text.toLowerCase();
  return tokens.some((t) => lower.includes(t.toLowerCase()));
}

/** True when source or translation matches any query token. */
export function segmentMatchesQuery(
  segment: { sourceText: string; translatedText: string },
  query: string,
): boolean {
  return (
    textMatchesQuery(segment.sourceText, query) ||
    textMatchesQuery(segment.translatedText, query)
  );
}

export type HighlightPart = { text: string; match: boolean };

/** Split `text` into alternating non-match / match parts for the given query. */
export function splitHighlightParts(
  text: string,
  query: string,
): HighlightPart[] {
  const tokens = queryTokens(query);
  if (!text || tokens.length === 0) {
    return text ? [{ text, match: false }] : [];
  }
  const pattern = new RegExp(`(${tokens.map(escapeRegExp).join("|")})`, "gi");
  const parts: HighlightPart[] = [];
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > last) {
      parts.push({ text: text.slice(last, index), match: false });
    }
    parts.push({ text: match[0], match: true });
    last = index + match[0].length;
  }
  if (last < text.length) {
    parts.push({ text: text.slice(last), match: false });
  }
  return parts.length > 0 ? parts : [{ text, match: false }];
}
