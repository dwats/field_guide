import { describe, expect, it } from "vitest";

import { isWebLink } from "./links";

describe("isWebLink", () => {
  it("offers http and https addresses", () => {
    expect(isWebLink("https://example.com/speedrun")).toBe(true);
    expect(isWebLink("HTTP://Example.com")).toBe(true);
  });

  it("refuses every other kind of link", () => {
    for (const href of [
      "file:///C:/Windows/notepad.exe",
      "javascript:alert(1)",
      "mailto:someone@example.com",
      "ms-settings:display",
      "media/abc.webp",
      "example.com",
      "https://",
      "https:///etc",
      "",
    ]) {
      expect(isWebLink(href), href).toBe(false);
    }
  });
});
