import { describe, expect, it } from "vitest";

import { moveHighlight, pickExplicitIndex, pickIndex, stepIndex } from "./combobox";

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

describe("stepIndex", () => {
  it("stops at both ends and enters from either side", () => {
    expect(stepIndex(0, -1, 3)).toBe(0);
    expect(stepIndex(2, 1, 3)).toBe(2);
    expect(stepIndex(1, 1, 3)).toBe(2);
    expect(stepIndex(-1, 1, 3)).toBe(0);
    expect(stepIndex(-1, -1, 3)).toBe(2);
    expect(stepIndex(0, 1, 0)).toBe(-1);
  });
});

describe("pickExplicitIndex", () => {
  it("takes an exact match without arrow keys", () => {
    expect(pickExplicitIndex(2, 0, 5, false)).toBe(2);
  });
  it("takes the highlighted row once the arrow keys reached it", () => {
    expect(pickExplicitIndex(-1, 3, 5, true)).toBe(3);
  });
  it("takes nothing for a partial entry the arrow keys did not touch", () => {
    expect(pickExplicitIndex(-1, 0, 5, false)).toBeNull();
  });
  it("takes nothing from an empty list", () => {
    expect(pickExplicitIndex(-1, 0, 0, true)).toBeNull();
  });
});
