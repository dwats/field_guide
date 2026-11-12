import { describe, expect, it } from "vitest";

import { sentenceCase } from "./sentence";

describe("sentenceCase", () => {
  it("raises the first letter of a core message", () => {
    expect(sentenceCase('a board named "X" already exists in this guide')).toBe(
      'A board named "X" already exists in this guide',
    );
  });

  it("leaves a message that already starts in upper case", () => {
    expect(sentenceCase("Already fine.")).toBe("Already fine.");
  });

  it("leaves a message that starts with a quoted name as written", () => {
    expect(sentenceCase('"notes" is not a tag')).toBe('"notes" is not a tag');
  });

  it("handles empty text and text without letters", () => {
    expect(sentenceCase("")).toBe("");
    expect(sentenceCase("404")).toBe("404");
  });
});
