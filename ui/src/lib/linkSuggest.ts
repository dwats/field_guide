// Link suggestions in the editor: the pure parts. Finding the open `[[` before the caret,
// shaping core's name matches into rows, and deciding what accepting a row inserts. The editor
// component owns the CodeMirror source and the call to core.
import { syntaxTree } from "@codemirror/language";
import type { EditorState } from "@codemirror/state";

import type { NameMatch } from "./api";

/** How many rows the list shows, the "new entry" row included. */
export const LINK_ROWS = 8;

/**
 * The link being typed, from the text of the current line before the caret: where its target
 * starts (just after `[[`) and the target typed so far. Null when the caret is not inside an
 * open link target: no `[[`, a `]` or `[` after it, or a `|` that starts the shown text.
 */
export function openLinkBefore(lineBefore: string): { start: number; query: string } | null {
  const m = /\[\[([^[\]|]*)$/.exec(lineBefore);
  return m ? { start: m.index + 2, query: m[1] } : null;
}

const CODE_NODES = new Set(["InlineCode", "CodeText", "CodeMark", "FencedCode", "CodeBlock", "CodeInfo"]);

/** True when `pos` is inside inline code or a code block in the markdown syntax tree. */
export function inCode(state: EditorState, pos: number): boolean {
  const inner = syntaxTree(state).resolveInner(pos, -1);
  for (let node: typeof inner | null = inner; node; node = node.parent) {
    if (CODE_NODES.has(node.name)) return true;
  }
  return false;
}

export interface LinkRow {
  key: string;
  /** The row's main text. */
  label: string;
  /** Muted text after the label: the alias that matched, or "stub". */
  detail?: string;
  /** The link target that accepting the row writes. */
  target: string;
}

const norm = (s: string) => s.trim().toLowerCase();

/**
 * Rows for the list: one per entry (a title match wins over an alias match of the same entry),
 * exact matches first, otherwise in core's order; then, when the typed text is not empty and no
 * title or alias equals it, a last row that keeps it as a link to an entry that does not exist yet.
 */
export function linkRows(query: string, matches: NameMatch[], limit = LINK_ROWS): LinkRow[] {
  const key = norm(query);
  const byEntry = new Map<string, NameMatch>();
  for (const m of matches) {
    const seen = byEntry.get(m.entry_id);
    if (!seen || (seen.is_alias && !m.is_alias)) byEntry.set(m.entry_id, m);
  }
  const exact = (m: NameMatch) => key !== "" && norm(m.matched) === key;
  const picked = [...byEntry.values()].sort((a, b) => Number(exact(b)) - Number(exact(a)));
  const hasExact = matches.some(exact);
  const newRow = key !== "" && !hasExact;
  const rows: LinkRow[] = picked.slice(0, newRow ? limit - 1 : limit).map((m) => ({
    key: m.entry_id,
    label: m.title,
    detail: m.is_alias && norm(m.matched) !== norm(m.title) ? `alias: ${m.matched}` : m.is_stub ? "stub" : undefined,
    target: m.title,
  }));
  if (newRow) {
    const typed = query.trim();
    rows.push({ key: "new-entry", label: `Link to a new entry "${typed}"`, target: typed });
  }
  return rows;
}

/**
 * What accepting a row writes, given the text of the line after the caret. The change starts at
 * the start of the link target and runs to the caret plus `replaceAfter`; `caret` is where the
 * caret lands, counted from the start of the target.
 *
 * - Nothing closes the link: write the target and `]]`.
 * - The rest of a closed link follows (`ten]]`, `ten|shown]]`): replace the rest of the old
 *   target, keep the closing brackets and any shown text, and put the caret after the `]]`.
 * - A single `]` follows: add the one missing bracket.
 */
export function linkInsertion(
  after: string,
  target: string,
): { insert: string; replaceAfter: number; caret: number } {
  const tail = /^[^[\]|]*/.exec(after)![0];
  const rest = after.slice(tail.length);
  if (rest.startsWith("]]")) {
    return { insert: target, replaceAfter: tail.length, caret: target.length + 2 };
  }
  if (rest.startsWith("|")) {
    const shown = /^\|[^[\]]*\]\]/.exec(rest);
    return {
      insert: target,
      replaceAfter: tail.length,
      caret: target.length + (shown ? shown[0].length : 0),
    };
  }
  if (rest.startsWith("]")) {
    return { insert: `${target}]`, replaceAfter: tail.length, caret: target.length + 2 };
  }
  return { insert: `${target}]]`, replaceAfter: 0, caret: target.length + 2 };
}
