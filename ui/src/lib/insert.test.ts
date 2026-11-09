import { describe, expect, it } from "vitest";

import { asOwnParagraph } from "./insert";

const img = "![a](media/x.webp)";

describe("asOwnParagraph", () => {
  it("pads a block so it stands alone wherever it lands", () => {
    expect(asOwnParagraph("", 0, img)).toBe(`${img}\n`);
    expect(asOwnParagraph("abc", 3, img)).toBe(`\n\n${img}\n`);
    expect(asOwnParagraph("hello world", 6, img)).toBe(`\n\n${img}\n\n`);
    expect(asOwnParagraph("a\nb", 2, img)).toBe(`\n${img}\n\n`);
    expect(asOwnParagraph("a\n\nb", 3, img)).toBe(`${img}\n\n`);
    expect(asOwnParagraph("a\n\n\n\nb", 3, img)).toBe(`${img}`);
    expect(asOwnParagraph("a\n", 2, img)).toBe(`\n${img}\n`);
  });

  it("produces a figure paragraph in the document", () => {
    const doc = "hello world";
    const insert = asOwnParagraph(doc, 6, img);
    expect(doc.slice(0, 6) + insert + doc.slice(6)).toBe(`hello \n\n${img}\n\nworld`);
  });
});
