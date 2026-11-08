import { describe, expect, it } from "vitest";

import type { TagNode } from "./api";
import {
  ancestorsOf,
  idsWithChildren,
  inScope,
  pruneTree,
  queryFor,
  sameScope,
  summaryLine,
} from "./sidebar";

function node(id: string, path: string, children: TagNode[] = []): TagNode {
  const name = path.split("/").pop() ?? path;
  return { tag: { id, scope_guide_id: "g", parent_id: null, name, color: null }, path, children };
}

const tree = [
  node("blocks", "Blocks", [node("natural", "Blocks/Natural"), node("ore", "Blocks/Ore")]),
  node("mobs", "Mobs", [node("hostile", "Mobs/Hostile")]),
];
const tags = [
  { id: "blocks", path: "Blocks", color: null },
  { id: "natural", path: "Blocks/Natural", color: null },
  { id: "ore", path: "Blocks/Ore", color: null },
];

describe("pruneTree", () => {
  it("keeps matches with their ancestors and leaves an empty query alone", () => {
    expect(pruneTree(tree, "")).toBe(tree);
    const pruned = pruneTree(tree, "ore");
    expect(pruned.map((n) => n.tag.id)).toEqual(["blocks"]);
    expect(pruned[0].children.map((n) => n.tag.id)).toEqual(["ore"]);
    expect(idsWithChildren(pruned)).toEqual(["blocks"]);
    expect(pruneTree(tree, "nothing")).toEqual([]);
  });

  it("keeps the whole subtree of a matching tag", () => {
    const pruned = pruneTree(tree, "blocks");
    expect(pruned[0].children.length).toBe(2);
  });
});

describe("ancestorsOf and inScope", () => {
  it("walks up to the root and decides scope membership through the tree", () => {
    expect(ancestorsOf(tree, "ore")).toEqual(["blocks"]);
    expect(ancestorsOf(tree, "blocks")).toEqual([]);
    expect(ancestorsOf(tree, "missing")).toEqual([]);
    expect(inScope({ kind: "tag", id: "blocks" }, ["ore"], tree)).toBe(true);
    expect(inScope({ kind: "tag", id: "mobs" }, ["ore"], tree)).toBe(false);
    expect(inScope({ kind: "untagged" }, [], tree)).toBe(true);
    expect(inScope({ kind: "untagged" }, ["ore"], tree)).toBe(false);
    expect(inScope({ kind: "all" }, ["ore"], tree)).toBe(true);
    expect(sameScope({ kind: "tag", id: "a" }, { kind: "tag", id: "b" })).toBe(false);
    expect(sameScope({ kind: "all" }, { kind: "all" })).toBe(true);
  });
});

describe("queryFor", () => {
  it("puts the scope tag first, resolves chips, and reports unknown ones", () => {
    const { query, unknownTags } = queryFor(
      { kind: "tag", id: "blocks" },
      { text: "stone", tags: ["Blocks/Ore", "Nope"] },
      true,
      tags,
    );
    expect(query).toEqual({ tag_ids: ["blocks", "ore"], text: "stone", stubs_only: true, untagged: false });
    expect(unknownTags).toEqual(["Nope"]);
    expect(queryFor({ kind: "untagged" }, { text: "", tags: [] }, false, tags).query).toEqual({
      tag_ids: [],
      text: null,
      stubs_only: false,
      untagged: true,
    });
  });
});

describe("summaryLine", () => {
  it("lists the parts in effect with a pluralised count", () => {
    expect(summaryLine("All entries", { text: "", tags: [] }, false, 124)).toBe("All entries · 124 entries");
    expect(summaryLine("Blocks/Natural", { text: "stone", tags: ["Ore"] }, true, 1)).toBe(
      'Blocks/Natural · "stone" · #Ore · stubs only · 1 entry',
    );
  });
});
