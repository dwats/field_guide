import { describe, expect, it } from "vitest";

import { hiddenHashRanges } from "./mediaNames";

const hash = "6a74" + "0".repeat(57) + "ad0";

describe("hiddenHashRanges", () => {
  it("hides the middle of a media hash, keeping four digits before and three after", () => {
    const text = `![Oak](media/${hash}.webp)`;
    const [range] = hiddenHashRanges(text);
    const shown = text.slice(0, range.from) + "…" + text.slice(range.to);
    expect(shown).toBe("![Oak](media/6a74…ad0.webp)");
  });

  it("finds every reference on a line", () => {
    const text = `![](media/${hash}.png) and ![](media/${hash}.jpg)`;
    expect(hiddenHashRanges(text)).toHaveLength(2);
  });

  it("leaves text that is not a stored media name alone", () => {
    expect(hiddenHashRanges("media/short.png and https://example.com/a.png")).toEqual([]);
    expect(hiddenHashRanges(`media/${hash.toUpperCase()}.png`)).toEqual([]);
  });
});
