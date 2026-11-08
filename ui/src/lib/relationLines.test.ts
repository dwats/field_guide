import { describe, expect, it } from "vitest";

import type { RelationView } from "./api";
import { bodySegments, otherEnd } from "./relationLines";

function view(section: RelationView["section"], segments: RelationView["segments"]): RelationView {
  return {
    relation: {} as RelationView["relation"],
    perspective: "subject",
    qualifier_id: null,
    section,
    label: "DROPS",
    inverted: false,
    external: false,
    text: segments.map((s) => s.text).join(""),
    segments,
  };
}

describe("bodySegments", () => {
  it("drops the verb slot under a group heading but keeps referenced-by lines whole", () => {
    const grouped = view("relations", [
      { text: "DROPS ", verb_slot: true },
      { text: "Cobblestone Block", entry_id: "n2" },
      { text: " · mined with " },
      { text: "Pickaxe", entry_id: "n3" },
    ]);
    expect(bodySegments(grouped).map((s) => s.text)).toEqual([
      "Cobblestone Block",
      " · mined with ",
      "Pickaxe",
    ]);
    const referenced = view("referenced_by", [
      { text: "mined with · " },
      { text: "Stone Block", entry_id: "n1" },
      { text: " DROPS " },
      { text: "Cobblestone Block", entry_id: "n2" },
    ]);
    expect(bodySegments(referenced)).toEqual(referenced.segments);
  });
});

describe("otherEnd", () => {
  it("finds the first linked entry that is not the one being read", () => {
    const v = view("relations", [
      { text: "MINES ", verb_slot: true },
      { text: "Stone Block", entry_id: "n1" },
      { text: " → " },
      { text: "Cobblestone Block", entry_id: "n2" },
    ]);
    expect(otherEnd(v, "n3")).toBe("n1");
    expect(otherEnd(v, "n1")).toBe("n2");
    expect(otherEnd(view("relations", [{ text: "ADJACENT TO ", verb_slot: true }]), "n1")).toBeNull();
  });
});
