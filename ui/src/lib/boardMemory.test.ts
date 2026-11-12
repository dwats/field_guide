import { describe, expect, it } from "vitest";

import {
  cardTab,
  chooseCardTab,
  forgetView,
  recallView,
  rememberView,
} from "./boardMemory";

describe("board memory", () => {
  it("gives each board back the pan and zoom it was left at", () => {
    expect(recallView("first")).toBeNull();
    rememberView("first", { x: 10, y: -20, k: 0.5 });
    rememberView("second", { x: 300, y: 40, k: 2 });
    rememberView("first", { x: 12, y: -20, k: 0.6 });
    expect(recallView("first")).toEqual({ x: 12, y: -20, k: 0.6 });
    expect(recallView("second")).toEqual({ x: 300, y: 40, k: 2 });
  });

  it("is not changed through a recalled copy", () => {
    rememberView("copy", { x: 1, y: 2, k: 1 });
    const view = recallView("copy")!;
    view.x = 99;
    expect(recallView("copy")).toEqual({ x: 1, y: 2, k: 1 });
  });

  it("drops a deleted board's view", () => {
    rememberView("gone", { x: 1, y: 2, k: 1 });
    forgetView("gone");
    expect(recallView("gone")).toBeNull();
  });

  it("starts the selected card on the Entry tab and keeps the last one chosen", () => {
    expect(cardTab()).toBe("entry");
    chooseCardTab("relations");
    expect(cardTab()).toBe("relations");
  });
});
