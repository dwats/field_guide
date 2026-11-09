import { describe, expect, it } from "vitest";

import type { EntrySummary, Guide } from "./api";
import { buildGroups, flatten, parseInput } from "./palette";

function entry(title: string): EntrySummary {
  return {
    id: title,
    guide_id: "g",
    title,
    is_stub: false,
    deleted_at: null,
    created_at: "",
    updated_at: "",
    tag_ids: ["t1"],
  };
}

const guides: Guide[] = [
  { id: "g", name: "Minecraft", description: "", cover_media_id: null, cover_crop: null, tag_ids: [], created_at: "", updated_at: "" },
  { id: "h", name: "Valheim", description: "", cover_media_id: null, cover_crop: null, tag_ids: [], created_at: "", updated_at: "" },
];
const commands = [
  { action: "new" as const, label: "New entry" },
  { action: "settings" as const, label: "Settings" },
];

describe("parseInput", () => {
  it("reads the > prefix", () => {
    expect(parseInput("> set")).toEqual({ query: "set", commandsOnly: true });
    expect(parseInput(" stone ")).toEqual({ query: "stone", commandsOnly: false });
  });
});

describe("buildGroups", () => {
  it("orders the groups and adds a create row inside a guide", () => {
    const groups = buildGroups("sto", {
      entries: [entry("Stone Block")],
      tagPathOf: () => "Blocks",
      commands,
      guides,
      inGuide: true,
      mac: false,
    });
    expect(groups.map((g) => g.heading)).toEqual(["Entries", "New"]);
    expect(flatten(groups).at(-1)).toEqual({ kind: "create", title: "sto" });
    expect(groups[0].rows[0]).toMatchObject({ kind: "entry", detail: "Blocks" });
  });

  it("skips the create row on an exact title and lists guides first outside a guide", () => {
    const inside = buildGroups("Stone Block", {
      entries: [entry("Stone Block")],
      tagPathOf: () => "",
      commands,
      guides,
      inGuide: true,
    });
    expect(inside.some((g) => g.heading === "New")).toBe(false);
    const outside = buildGroups("", { entries: [], tagPathOf: () => "", commands, guides, inGuide: false, mac: false });
    expect(outside.map((g) => g.heading)).toEqual(["Guides", "Commands"]);
    expect(outside[1].rows[0]).toMatchObject({ kind: "command", keys: "Ctrl+N" });
  });

  it("limits to commands after >", () => {
    const groups = buildGroups("> sett", { entries: [entry("Settings Page")], tagPathOf: () => "", commands, guides, inGuide: true });
    expect(groups.map((g) => g.heading)).toEqual(["Commands"]);
    expect(groups[0].rows).toHaveLength(1);
  });
});
