// Pure helpers behind the sidebar: pruning the tag tree to a search, ancestor paths, scope
// membership, the entry query a scope produces, and the summary line the guide page shows.

import type { EntryCounts, EntryQuery, Id, TagNode } from "./api";
import type { ParsedFilter } from "./filter";
import { findByPath, type TagOption } from "./tags";

/** What the sidebar lists: every entry, one tag with its subtree, or entries with no tag. */
export type Scope = { kind: "all" } | { kind: "tag"; id: Id } | { kind: "untagged" };

export function tagScope(id: Id): Scope {
  return { kind: "tag", id };
}

export function sameScope(a: Scope, b: Scope): boolean {
  return a.kind === "tag" ? b.kind === "tag" && a.id === b.id : a.kind === b.kind;
}

/**
 * The tree reduced to tags whose path contains `query`, case-insensitively, plus the ancestors
 * that lead to them. A matching tag keeps its whole subtree. An empty query is the whole tree.
 */
export function pruneTree(nodes: TagNode[], query: string): TagNode[] {
  const key = query.trim().toLowerCase();
  if (!key) return nodes;
  const walk = (list: TagNode[]): TagNode[] =>
    list.flatMap((node) => {
      if (node.path.toLowerCase().includes(key)) return [node];
      const children = walk(node.children);
      return children.length > 0 ? [{ ...node, children }] : [];
    });
  return walk(nodes);
}

/** Ids of every node with children, so a search opens the paths to its matches. */
export function idsWithChildren(nodes: TagNode[]): Id[] {
  const out: Id[] = [];
  const walk = (list: TagNode[]) => {
    for (const node of list) {
      if (node.children.length > 0) {
        out.push(node.tag.id);
        walk(node.children);
      }
    }
  };
  walk(nodes);
  return out;
}

/** Ancestor ids of a tag, root first; empty for a top-level or unknown tag. */
export function ancestorsOf(nodes: TagNode[], id: Id): Id[] {
  const walk = (list: TagNode[], trail: Id[]): Id[] | null => {
    for (const node of list) {
      if (node.tag.id === id) return trail;
      const found = walk(node.children, [...trail, node.tag.id]);
      if (found) return found;
    }
    return null;
  };
  return walk(nodes, []) ?? [];
}

/** Whether an entry with these tags falls inside the scope: the tag itself or one under it. */
export function inScope(scope: Scope, tagIds: Id[], nodes: TagNode[]): boolean {
  if (scope.kind === "all") return true;
  if (scope.kind === "untagged") return tagIds.length === 0;
  return tagIds.some((t) => t === scope.id || ancestorsOf(nodes, t).includes(scope.id));
}

export function subtreeCount(counts: EntryCounts | null, id: Id): number {
  return counts?.by_tag.find((c) => c.tag_id === id)?.subtree ?? 0;
}

export function scopeLabel(scope: Scope, tags: TagOption[]): string {
  if (scope.kind === "all") return "All entries";
  if (scope.kind === "untagged") return "Untagged";
  return tags.find((t) => t.id === scope.id)?.path ?? "Tag";
}

/** The query core answers for a scope, the parsed search line, and the stubs toggle. */
export function queryFor(
  scope: Scope,
  parsed: ParsedFilter,
  stubsOnly: boolean,
  tags: TagOption[],
): { query: EntryQuery; unknownTags: string[] } {
  const resolved = parsed.tags.map((path) => ({ path, option: findByPath(tags, path) }));
  // An untagged entry carries no tag, so chips cannot narrow that scope; they are ignored there.
  const tag_ids =
    scope.kind === "untagged" ? [] : resolved.flatMap((r) => (r.option ? [r.option.id] : []));
  if (scope.kind === "tag") tag_ids.unshift(scope.id);
  return {
    query: {
      tag_ids,
      text: parsed.text || null,
      stubs_only: stubsOnly,
      untagged: scope.kind === "untagged",
    },
    unknownTags: resolved.filter((r) => !r.option).map((r) => r.path),
  };
}

/**
 * `Blocks/Natural · "stone" · #Ore · 6 entries`: the guide page's fixed summary line. A null label
 * is the whole guide (All entries), which the count already says, so the line starts after it.
 */
export function summaryLine(
  label: string | null,
  parsed: ParsedFilter,
  stubsOnly: boolean,
  count: number,
): string {
  const parts = label === null ? [] : [label];
  if (parsed.text) parts.push(`"${parsed.text}"`);
  for (const tag of parsed.tags) parts.push(`#${tag}`);
  if (stubsOnly) parts.push("stubs only");
  parts.push(`${count} ${count === 1 ? "entry" : "entries"}`);
  return parts.join(" · ");
}

/** Pages whose content is the sidebar's list, so the scope row is the selected row there. */
export function isListPage(view: string): boolean {
  return view === "guide" || view === "entry";
}

/**
 * The fixed row under a tag-scoped search: how many more entries match in All entries. Null when
 * the scope is not a tag or the search has no text, so the row is absent.
 */
export function widenRow(
  scope: Scope,
  text: string,
  scopedCount: number,
  allCount: number | null,
): { more: number; label: string } | null {
  if (scope.kind !== "tag" || !text.trim() || allCount === null) return null;
  const more = Math.max(0, allCount - scopedCount);
  return { more, label: more === 0 ? "No more in All entries" : `${more} more in All entries` };
}

/** Row keys of the guide tree's fixed nodes; tag rows use the tag id, board rows `board:<id>`. */
export const ROW_ALL = "all";
export const ROW_UNTAGGED = "untagged";
export const ROW_BOARDS = "boards";
export const ROW_TRASH = "trash";

export function boardRowKey(id: Id): string {
  return `board:${id}`;
}

/** The rows the guide tree shows, top to bottom: All entries, tags, Untagged, Boards, Trash. */
export function visibleTreeKeys(
  nodes: TagNode[],
  isOpen: (id: Id) => boolean,
  boardIds: Id[] | null,
): string[] {
  const out = [ROW_ALL];
  const walk = (list: TagNode[]) => {
    for (const node of list) {
      out.push(node.tag.id);
      if (node.children.length > 0 && isOpen(node.tag.id)) walk(node.children);
    }
  };
  walk(nodes);
  out.push(ROW_UNTAGGED, ROW_BOARDS);
  if (boardIds) out.push(...boardIds.map(boardRowKey));
  out.push(ROW_TRASH);
  return out;
}

/**
 * The one row of a list that Tab reaches (a roving tab stop): the row last focused while it is
 * still shown, else the selected row, else the first row.
 */
export function rovingStop(
  keys: string[],
  remembered: string | null,
  selected: string | null,
): string | null {
  if (remembered && keys.includes(remembered)) return remembered;
  if (selected && keys.includes(selected)) return selected;
  return keys[0] ?? null;
}
