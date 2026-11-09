import { describe, expect, it } from "vitest";

import { altFromName, extensionOf, fileName, formatBytes, imageReference, isImagePath } from "./media";

const media = { sha256: "ab".repeat(32), ext: "webp" };

describe("references and names", () => {
  it("builds the body reference and the file name", () => {
    expect(fileName(media)).toBe(`${"ab".repeat(32)}.webp`);
    expect(imageReference(media, "A [cave]")).toBe(`![A cave](media/${"ab".repeat(32)}.webp)`);
    expect(imageReference(media)).toBe(`![](media/${"ab".repeat(32)}.webp)`);
  });

  it("reads extensions and recognizes image paths on both separators", () => {
    expect(extensionOf("C:\\shots\\Cave.PNG")).toBe("png");
    expect(extensionOf("/tmp/notes")).toBe("");
    expect(extensionOf(".hidden")).toBe("");
    expect(isImagePath("a/b/c.jpeg")).toBe(true);
    expect(isImagePath("a/b/c.txt")).toBe(false);
  });

  it("makes a caption from a meaningful file name only", () => {
    expect(altFromName("C:\\shots\\dark_cave-entrance.png")).toBe("dark cave entrance");
    expect(altFromName("image.png")).toBe("");
    expect(altFromName("Screenshot 12.png")).toBe("");
  });
});

describe("formatBytes", () => {
  it("picks a unit", () => {
    expect(formatBytes(900)).toBe("900 B");
    expect(formatBytes(20 * 1024)).toBe("20 KB");
    expect(formatBytes(3.25 * 1024 * 1024)).toBe("3.3 MB");
  });
});
