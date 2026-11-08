import { describe, expect, it } from "vitest";

import { moveHighlight, pickIndex } from "./combobox";

describe("moveHighlight", () => {
  it("wraps at both ends and tolerates an empty list", () => {
    expect(moveHighlight(0, 1, 3)).toBe(1);
    expect(moveHighlight(2, 1, 3)).toBe(0);
    expect(moveHighlight(0, -1, 3)).toBe(2);
    expect(moveHighlight(5, 1, 0)).toBe(0);
  });
});

describe("pickIndex", () => {
  it("takes the exact match first, then the highlight, and nothing from an empty list", () => {
    expect(pickIndex(2, 0, 4)).toBe(2);
    expect(pickIndex(-1, 3, 4)).toBe(3);
    expect(pickIndex(-1, 9, 4)).toBe(3);
    expect(pickIndex(-1, -2, 4)).toBe(0);
    expect(pickIndex(0, 0, 0)).toBeNull();
  });
});
