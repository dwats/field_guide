import { describe, expect, it } from "vitest";

import { verbSlot, when } from "./format";

describe("when", () => {
  it("renders local time to the minute in one fixed shape", () => {
    const local = new Date(2026, 0, 5, 9, 7, 59);
    expect(when(local.toISOString())).toBe("2026-01-05 09:07");
  });

  it("passes unparseable input through", () => {
    expect(when("not a date")).toBe("not a date");
  });
});

describe("verbSlot", () => {
  it("reads uppercase", () => {
    expect(verbSlot("dropped by")).toBe("DROPPED BY");
  });
});
