import { describe, expect, it } from "vitest";

import type { TagNode } from "./api";
import { colorVar, findByPath, flattenTags, pathsOf } from "./tags";

function node(id: string, path: string, color: string | null, children: TagNode[] = []): TagNode {
  const name = path.split("/").pop() ?? path;
  return { tag: { id, scope_guide_id: "g", parent_id: null, name, color }, path, children };
}

const tree = [node("blocks", "Blocks", "blue", [node("ore", "Blocks/Ore", null)]), node("mobs", "Mobs", "red")];

describe("flattenTags and findByPath", () => {
  it("lists parents before children and matches paths case-insensitively", () => {
    const flat = flattenTags(tree);
    expect(flat.map((t) => t.path)).toEqual(["Blocks", "Blocks/Ore", "Mobs"]);
    expect(findByPath(flat, " blocks/ORE ")?.id).toBe("ore");
    expect(findByPath(flat, "Nope")).toBeUndefined();
  });
});

describe("pathsOf", () => {
  it("joins known paths and marks unknown ids", () => {
    const flat = flattenTags(tree);
    expect(pathsOf(flat, ["ore", "mobs"])).toBe("Blocks/Ore, Mobs");
    expect(pathsOf(flat, ["gone"])).toBe("…");
    expect(pathsOf(flat, [])).toBe("");
  });
});

describe("colorVar", () => {
  it("accepts the presets only", () => {
    expect(colorVar("blue")).toBe("var(--tag-blue)");
    expect(colorVar("chartreuse")).toBeNull();
    expect(colorVar(null)).toBeNull();
    expect(colorVar(undefined)).toBeNull();
  });
});
