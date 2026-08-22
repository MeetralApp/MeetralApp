import { describe, expect, it } from "vitest";

import {
  anchorColumnLabel,
  blockLabel,
  summaryDocHasContent,
} from "./summaryDisplay";

describe("summaryDocHasContent", () => {
  it("accepts TipTap docs with blocks", () => {
    expect(
      summaryDocHasContent(
        JSON.stringify({
          type: "doc",
          content: [{ type: "paragraph", content: [{ type: "text", text: "Hi" }] }],
        }),
      ),
    ).toBe(true);
  });

  it("rejects empty or non-doc JSON", () => {
    expect(summaryDocHasContent("{}")).toBe(false);
    expect(summaryDocHasContent('{"type":"doc","content":[]}')).toBe(false);
    expect(summaryDocHasContent("not-json")).toBe(false);
  });
});

describe("blockLabel", () => {
  it("returns Vietnamese labels", () => {
    expect(blockLabel("overview", "vi")).toBe("Tổng quan");
    expect(blockLabel("actionItems", "en")).toBe("Action items");
  });
});

describe("anchorColumnLabel", () => {
  it("maps directions to column labels", () => {
    expect(anchorColumnLabel("outbound")).toBe("You");
    expect(anchorColumnLabel("inbound")).toBe("Meeting");
  });
});
