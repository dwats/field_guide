import { describe, expect, it } from "vitest";

import { bodyFromReferences, titleFromImages } from "./pasteEntry";

describe("titleFromImages", () => {
  it("takes the first meaningful name and capitalizes it", () => {
    expect(titleFromImages(["image.png", "C:\\shots\\crypt_door-north.png"])).toBe("Crypt door north");
  });

  it("gives nothing for clipboard and screenshot names", () => {
    expect(titleFromImages(["image.png"])).toBe("");
    expect(titleFromImages(["Screenshot 3.png", "clipboard.webp"])).toBe("");
    expect(titleFromImages([])).toBe("");
  });
});

describe("bodyFromReferences", () => {
  it("puts each image on its own line and ends with a newline", () => {
    expect(bodyFromReferences(["![](media/a.png)", "![Cave](media/b.png)"])).toBe(
      "![](media/a.png)\n![Cave](media/b.png)\n",
    );
  });

  it("is empty without images", () => {
    expect(bodyFromReferences([])).toBe("");
  });
});
