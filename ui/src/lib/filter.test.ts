import { describe, expect, it } from "vitest";

import { formatFilter, parseFilter } from "./filter";

describe("parseFilter", () => {
  it("separates free text from tag chips", () => {
    expect(parseFilter("stone #Blocks/Natural pick")).toEqual({
      text: "stone pick",
      tags: ["Blocks/Natural"],
    });
  });

  it("keeps spaces inside quoted tags and ignores empty ones", () => {
    expect(parseFilter('#"Consumables/Potions of Healing" #"  " x')).toEqual({
      text: "x",
      tags: ["Consumables/Potions of Healing"],
    });
  });

  it("returns empty parts for blank input", () => {
    expect(parseFilter("   ")).toEqual({ text: "", tags: [] });
  });
});

describe("formatFilter", () => {
  it("round-trips through parseFilter", () => {
    const parsed = { text: "oak log", tags: ["Blocks/Natural", "Food and Drink"] };
    expect(parseFilter(formatFilter(parsed))).toEqual(parsed);
  });
});
