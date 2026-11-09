// Helpers over tag trees from `core`: flat path lookups for pickers and filters.

import type { Id, TagNode } from "./api";

export interface TagOption {
  id: Id;
  path: string;
  color: string | null;
}

/** Every tag in the tree with its display path, parents before children. */
export function flattenTags(nodes: TagNode[]): TagOption[] {
  const out: TagOption[] = [];
  const walk = (list: TagNode[]) => {
    for (const node of list) {
      out.push({ id: node.tag.id, path: node.path, color: node.tag.color });
      walk(node.children);
    }
  };
  walk(nodes);
  return out;
}

/** The colour names a tag can carry; each has a `--tag-<name>` token in app.css. */
export const TAG_COLORS = [
  "red",
  "orange",
  "yellow",
  "green",
  "teal",
  "blue",
  "purple",
  "pink",
  "gray",
] as const;

/** The CSS value for a tag's colour, or null when it has none or an unknown one. */
export function colorVar(color: string | null | undefined): string | null {
  return color && (TAG_COLORS as readonly string[]).includes(color) ? `var(--tag-${color})` : null;
}

/** Display paths for a list of tag ids, comma-separated; an unknown id shows as an ellipsis. */
export function pathsOf(tags: TagOption[], ids: Id[]): string {
  return ids.map((id) => tags.find((t) => t.id === id)?.path ?? "…").join(", ");
}

/** Finds a tag by path, case-insensitively, the way `core` matches names. */
export function findByPath(entries: TagOption[], path: string): TagOption | undefined {
  const wanted = path.trim().toLowerCase();
  return entries.find((e) => e.path.toLowerCase() === wanted);
}
