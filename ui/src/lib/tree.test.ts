import { describe, expect, it } from "vitest";

import type { TagNode } from "./api";
import { findNode, subtreeIds, visibleRows, jumpIndex } from "./tree";

function node(id: string, children: TagNode[] = []): TagNode {
  return { tag: { id, scope_guide_id: "g", parent_id: null, name: id, color: null }, path: id, children };
}

const tree = [node("blocks", [node("natural", [node("stone")]), node("ore")]), node("mobs")];

describe("visibleRows", () => {
  it("shows children only under expanded nodes, with depths", () => {
    expect(visibleRows(tree, new Set()).map((r) => r.node.tag.id)).toEqual(["blocks", "mobs"]);
    const open = visibleRows(tree, new Set(["blocks"]));
    expect(open.map((r) => [r.node.tag.id, r.depth])).toEqual([
      ["blocks", 0],
      ["natural", 1],
      ["ore", 1],
      ["mobs", 0],
    ]);
  });
});

describe("findNode and subtreeIds", () => {
  it("finds nested nodes and lists a subtree", () => {
    expect(findNode(tree, "stone")?.path).toBe("stone");
    expect(findNode(tree, "nope")).toBeNull();
    expect(subtreeIds(tree[0])).toEqual(["blocks", "natural", "stone", "ore"]);
  });
});

describe("jumpIndex", () => {
  it("goes to the next row starting with the letter, wrapping around", () => {
    const names = ["All entries", "Blocks", "Biomes", "Tools", "Untagged", "Trash"];
    expect(jumpIndex(names, 0, "b")).toBe(1);
    expect(jumpIndex(names, 1, "B")).toBe(2);
    expect(jumpIndex(names, 2, "b")).toBe(1);
    expect(jumpIndex(names, 3, "t")).toBe(5);
    expect(jumpIndex(names, 5, "t")).toBe(3);
    expect(jumpIndex(names, 4, "u")).toBe(4);
    expect(jumpIndex(names, 0, "z")).toBe(-1);
  });
});
