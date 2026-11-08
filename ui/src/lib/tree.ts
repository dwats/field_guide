// Tree helpers for the tag editor: the rows on screen given what is expanded, and a node's
// descendants (which a tag cannot be moved under).

import type { Id, TagNode } from "./api";

export interface VisibleRow {
  node: TagNode;
  depth: number;
}

export function visibleRows(nodes: TagNode[], expanded: ReadonlySet<Id>): VisibleRow[] {
  const out: VisibleRow[] = [];
  const walk = (list: TagNode[], depth: number) => {
    for (const node of list) {
      out.push({ node, depth });
      if (node.children.length > 0 && expanded.has(node.tag.id)) walk(node.children, depth + 1);
    }
  };
  walk(nodes, 0);
  return out;
}

export function findNode(nodes: TagNode[], id: Id): TagNode | null {
  for (const node of nodes) {
    if (node.tag.id === id) return node;
    const found = findNode(node.children, id);
    if (found) return found;
  }
  return null;
}

/** The node's own id and every id under it. */
export function subtreeIds(node: TagNode): Id[] {
  return [node.tag.id, ...node.children.flatMap(subtreeIds)];
}
