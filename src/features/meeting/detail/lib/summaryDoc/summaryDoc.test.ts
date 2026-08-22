import { describe, expect, it } from "vitest";
import type { JSONContent } from "@tiptap/core";

import { citationLabel } from "./citationExtension";
import { citeItemToAttrs } from "./citationSuggestion";
import { docToProse } from "./docToProse";
import { extractCiteSegmentIds } from "./extractCiteSegmentIds";
import { seedFromBrief } from "./seedFromBrief";
import { summaryStarterKit } from "./summaryKit";
import { withCitationSnippets } from "./withCitationSnippets";
import {
  ANCHOR_SNIPPET_CAP,
  segmentSnippetText,
} from "@/features/meeting/library/lib/segmentSnippet";
import {
  STRUCTURE_SUGGESTION_ITEMS,
  filterStructureItems,
} from "./structureSuggestion";
import type { MeetingBriefSummary } from "../summaryTypes";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

const segments: TranscriptSegment[] = [
  {
    id: "seg-out",
    meetingId: "m1",
    direction: "outbound",
    sequence: 1,
    sourceText: "We decided on budget",
    translatedText: "",
    startedAtMs: 228_000,
    endedAtMs: 230_000,
    connectionGap: false,
  },
  {
    id: "seg-in",
    meetingId: "m1",
    direction: "inbound",
    sequence: 2,
    sourceText: "Agreed",
    translatedText: "",
    startedAtMs: 65_000,
    endedAtMs: 66_000,
    connectionGap: false,
  },
];

const brief: MeetingBriefSummary = {
  schemaVersion: 2,
  overview: {
    points: [
      {
        id: "p1",
        text: "Budget approved",
        segmentRefs: [{ direction: "outbound", sequence: 1 }],
      },
    ],
  },
  keyPoints: { points: [] },
  decisions: {
    points: [
      {
        id: "p2",
        text: "Ship next week",
        segmentRefs: [
          { direction: "inbound", sequence: 2 },
          { direction: "outbound", sequence: 1 },
        ],
      },
    ],
  },
  actionItems: {
    points: [
      {
        id: "p3",
        text: "Send proposal",
        owner: "Alex",
        due: "Fri",
        segmentRefs: [],
      },
    ],
  },
  openQuestions: { points: [] },
};

describe("citationLabel", () => {
  it("formats G5 chip text", () => {
    expect(
      citationLabel({ direction: "outbound", startedAtMs: 228_000 }),
    ).toBe("3:48·You");
    expect(
      citationLabel({ direction: "inbound", startedAtMs: 65_000 }),
    ).toBe("1:05·Meeting");
  });
});

describe("seedFromBrief", () => {
  it("builds headings, bullets, and citation atoms", () => {
    const doc = seedFromBrief(brief, segments, "en");
    expect(doc.type).toBe("doc");
    const cites = extractCiteSegmentIds(doc);
    expect(cites).toEqual(["seg-out", "seg-in"]);
    const prose = docToProse(doc);
    expect(prose).toContain("### Overview");
    expect(prose).toContain("Budget approved");
    expect(prose).toContain("3:48·You");
    expect(prose).toContain("Owner: Alex");
    expect(prose).toContain("Due: Fri");
  });

  it("captures the segment snippet into each citation atom", () => {
    const doc = seedFromBrief(brief, segments, "en");
    const atoms: JSONContent[] = [];
    const walk = (node: JSONContent) => {
      if (node.type === "citation") atoms.push(node);
      node.content?.forEach(walk);
    };
    walk(doc);
    const snippetBySegmentId = Object.fromEntries(
      atoms.map((atom) => [atom.attrs?.segmentId, atom.attrs?.snippet]),
    );
    expect(snippetBySegmentId).toEqual({
      "seg-out": "We decided on budget",
      "seg-in": "Agreed",
    });
  });
});

describe("segmentSnippetText", () => {
  it("prefers the translated text and collapses whitespace", () => {
    expect(
      segmentSnippetText({
        sourceText: "raw  text",
        translatedText: "bản\n dịch",
      }),
    ).toBe("bản dịch");
  });

  it("falls back to source text and caps at maxLen", () => {
    const long = "x".repeat(ANCHOR_SNIPPET_CAP + 10);
    const snippet = segmentSnippetText(
      { sourceText: long, translatedText: "" },
      ANCHOR_SNIPPET_CAP,
    );
    expect(snippet).toHaveLength(ANCHOR_SNIPPET_CAP);
  });
});

describe("citeItemToAttrs", () => {
  it("carries the snippet so doc-atom tooltips show transcript text", () => {
    const attrs = citeItemToAttrs({
      segment: segments[1],
      label: "1:05 · Meeting",
      snippet: "Agreed",
    });
    expect(attrs).toEqual({
      segmentId: "seg-in",
      direction: "inbound",
      startedAtMs: 65_000,
      snippet: "Agreed",
    });
  });
});

describe("withCitationSnippets", () => {
  const legacyDoc: JSONContent = {
    type: "doc",
    content: [
      {
        type: "paragraph",
        content: [
          {
            type: "citation",
            attrs: { segmentId: "seg-out", direction: "outbound", startedAtMs: 228_000 },
          },
          {
            type: "citation",
            attrs: {
              segmentId: "seg-in",
              direction: "inbound",
              startedAtMs: 65_000,
              snippet: "kept",
            },
          },
          {
            type: "citation",
            attrs: { segmentId: "gone", direction: "inbound", startedAtMs: 0 },
          },
        ],
      },
    ],
  };

  it("fills only missing snippets from resolvable segments", () => {
    const enriched = withCitationSnippets(legacyDoc, segments);
    const cites = enriched.content?.[0]?.content ?? [];
    expect(cites[0]?.attrs?.snippet).toBe("We decided on budget");
    expect(cites[1]?.attrs?.snippet).toBe("kept");
    expect(cites[2]?.attrs?.snippet).toBeUndefined();
  });

  it("returns the original doc when nothing changes", () => {
    expect(withCitationSnippets(legacyDoc, [])).toBe(legacyDoc);
    const doc: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "hi" }] }],
    };
    expect(withCitationSnippets(doc, segments)).toBe(doc);
  });
});

describe("summaryStarterKit", () => {
  it("narrows vocabulary to heading L3, bullets, bold", () => {
    const kit = summaryStarterKit();
    const options = kit.options as {
      heading: { levels: number[] } | false;
      blockquote: false | object;
      italic: false | object;
      orderedList: false | object;
      bold: false | object;
      bulletList: false | object;
    };
    expect(options.heading).toMatchObject({ levels: [3] });
    expect(options.blockquote).toBe(false);
    expect(options.italic).toBe(false);
    expect(options.orderedList).toBe(false);
    expect(options.bold).not.toBe(false);
    expect(options.bulletList).not.toBe(false);
  });
});

describe("filterStructureItems", () => {
  it("returns section and bullet by default", () => {
    expect(filterStructureItems("").map((i) => i.id)).toEqual([
      "section",
      "bullet",
    ]);
    expect(STRUCTURE_SUGGESTION_ITEMS).toHaveLength(2);
  });

  it("filters by keyword", () => {
    expect(filterStructureItems("head").map((i) => i.id)).toEqual(["section"]);
    expect(filterStructureItems("ul").map((i) => i.id)).toEqual(["bullet"]);
    expect(filterStructureItems("zzz")).toEqual([]);
  });
});

describe("docToProse + extractCiteSegmentIds", () => {
  it("dedupes cites and keeps prose labels", () => {
    const doc = {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            { type: "text", text: "Note " },
            {
              type: "citation",
              attrs: {
                segmentId: "a",
                direction: "outbound",
                startedAtMs: 0,
              },
            },
            {
              type: "citation",
              attrs: {
                segmentId: "a",
                direction: "outbound",
                startedAtMs: 0,
              },
            },
            {
              type: "citation",
              attrs: {
                segmentId: "b",
                direction: "inbound",
                startedAtMs: 1000,
              },
            },
          ],
        },
      ],
    };
    expect(extractCiteSegmentIds(doc)).toEqual(["a", "b"]);
    expect(docToProse(doc)).toBe("Note 0:00·You0:00·You0:01·Meeting");
  });
});
