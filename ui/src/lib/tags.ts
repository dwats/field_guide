// Helpers over tag trees from `core`: flat path lookups for pickers and filters.

import type { Id, TagNode } from "./api";

export interface TagEntry {
  id: Id;
  path: string;
  color: string | null;
}

/** Every tag in the tree with its display path, parents before children. */
export function flattenTags(nodes: TagNode[]): TagEntry[] {
  const out: TagEntry[] = [];
  const walk = (list: TagNode[]) => {
    for (const node of list) {
      out.push({ id: node.tag.id, path: node.path, color: node.tag.color });
      walk(node.children);
    }
  };
  walk(nodes);
  return out;
}

/** Finds a tag by path, case-insensitively, the way `core` matches names. */
export function findByPath(entries: TagEntry[], path: string): TagEntry | undefined {
  const wanted = path.trim().toLowerCase();
  return entries.find((e) => e.path.toLowerCase() === wanted);
}
