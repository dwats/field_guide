import { describe, expect, it } from "vitest";

import { outline } from "./contents";

describe("outline", () => {
  it("measures depth from the shallowest heading", () => {
    expect(
      outline([
        { level: 2, text: "Drops", id: "drops" },
        { level: 3, text: "Rare", id: "rare" },
        { level: 2, text: "Where found", id: "where-found" },
      ]),
    ).toEqual([
      { id: "drops", text: "Drops", depth: 0 },
      { id: "rare", text: "Rare", depth: 1 },
      { id: "where-found", text: "Where found", depth: 0 },
    ]);
    expect(outline([])).toEqual([]);
  });
});
