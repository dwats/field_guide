import { describe, expect, it } from "vitest";

import type { TagNode } from "./api";
import {
  ancestorsOf,
  idsWithChildren,
  inScope,
  pruneTree,
  queryFor,
  sameScope,
  scopeLabel,
  subtreeCount,
  summaryLine,
  isListPage,
  rovingStop,
  visibleTreeKeys,
  widenRow,
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

  it("ignores chips in the untagged scope", () => {
    const { query } = queryFor({ kind: "untagged" }, { text: "", tags: ["Blocks/Ore"] }, false, tags);
    expect(query.tag_ids).toEqual([]);
    expect(query.untagged).toBe(true);
  });
});

describe("scopeLabel and subtreeCount", () => {
  it("names scopes and reads subtree counts with zero for unknown tags", () => {
    expect(scopeLabel({ kind: "all" }, tags)).toBe("All entries");
    expect(scopeLabel({ kind: "untagged" }, tags)).toBe("Untagged");
    expect(scopeLabel({ kind: "tag", id: "ore" }, tags)).toBe("Blocks/Ore");
    expect(scopeLabel({ kind: "tag", id: "gone" }, tags)).toBe("Tag");
    const counts = { total: 3, untagged: 1, trashed: 0, trashed_boards: 0, by_tag: [{ tag_id: "ore", direct: 1, subtree: 2 }] };
    expect(subtreeCount(counts, "ore")).toBe(2);
    expect(subtreeCount(counts, "blocks")).toBe(0);
    expect(subtreeCount(null, "ore")).toBe(0);
  });
});

describe("summaryLine", () => {
  it("lists the parts in effect with a pluralised count", () => {
    expect(summaryLine(null, { text: "", tags: [] }, false, 124)).toBe("124 entries");
    expect(summaryLine(null, { text: "stone", tags: [] }, false, 6)).toBe('"stone" · 6 entries');
    expect(summaryLine("Untagged", { text: "", tags: [] }, false, 3)).toBe("Untagged · 3 entries");
    expect(summaryLine("Blocks/Natural", { text: "stone", tags: ["Ore"] }, true, 1)).toBe(
      'Blocks/Natural · "stone" · #Ore · stubs only · 1 entry',
    );
  });
});

describe("widenRow", () => {
  it("counts the matches outside a tag scope while the search has text", () => {
    expect(widenRow({ kind: "tag", id: "ore" }, "iron", 1, 5)).toEqual({ more: 4, label: "4 more in All entries" });
    expect(widenRow({ kind: "tag", id: "ore" }, "iron", 2, 2)).toEqual({ more: 0, label: "No more in All entries" });
  });

  it("is absent without text, outside a tag scope, or before the count arrives", () => {
    expect(widenRow({ kind: "tag", id: "ore" }, "  ", 1, 5)).toBeNull();
    expect(widenRow({ kind: "all" }, "iron", 1, 5)).toBeNull();
    expect(widenRow({ kind: "untagged" }, "iron", 1, 5)).toBeNull();
    expect(widenRow({ kind: "tag", id: "ore" }, "iron", 1, null)).toBeNull();
  });
});

describe("visibleTreeKeys", () => {
  it("lists the shown rows with fixed nodes first and last, boards only when open", () => {
    const open = new Set(["blocks"]);
    expect(visibleTreeKeys(tree, (id) => open.has(id), null)).toEqual([
      "all", "blocks", "natural", "ore", "mobs", "untagged", "boards", "trash",
    ]);
    expect(visibleTreeKeys([], () => false, ["b1", "b2"])).toEqual([
      "all", "untagged", "boards", "board:b1", "board:b2", "trash",
    ]);
  });
});

describe("rovingStop", () => {
  const keys = ["all", "blocks", "untagged", "trash"];
  it("prefers the last focused row, then the selected row, then the first", () => {
    expect(rovingStop(keys, "untagged", "blocks")).toBe("untagged");
    expect(rovingStop(keys, "gone", "blocks")).toBe("blocks");
    expect(rovingStop(keys, null, "hidden")).toBe("all");
    expect(rovingStop([], null, null)).toBeNull();
  });
});

describe("isListPage", () => {
  it("treats the guide and entry pages as the list and every other page as not", () => {
    expect(isListPage("guide")).toBe(true);
    expect(isListPage("entry")).toBe(true);
    for (const view of ["trash", "tags", "boards", "board", "settings"]) expect(isListPage(view)).toBe(false);
  });
});
