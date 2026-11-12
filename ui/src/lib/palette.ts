// What the command palette lists (UX review 4.7): entries of this guide, commands with their
// shortcuts, guides, and a row that creates the typed entry. Pure, so it is tested without a DOM.

import type { EntrySummary, Guide } from "./api";
import { displayKeys, shortcutFor, type Action } from "./keys";

export interface Command {
  action: Action;
  label: string;
  /** The chord to show in place of the map's, or null for none; absent means the map's. */
  keys?: string | null;
}

export type Row =
  | { kind: "entry"; entry: EntrySummary; detail: string }
  | { kind: "command"; command: Command; keys: string | null }
  | { kind: "guide"; guide: Guide }
  | { kind: "create"; title: string };

export interface Group {
  heading: string;
  rows: Row[];
}

/** A leading `>` limits the list to commands, as in VS Code. */
export function parseInput(text: string): { query: string; commandsOnly: boolean } {
  const trimmed = text.trim();
  if (trimmed.startsWith(">")) return { query: trimmed.slice(1).trim(), commandsOnly: true };
  return { query: trimmed, commandsOnly: false };
}

export interface Sources {
  /** Entries already filtered by core's search for the query, best first. */
  entries: EntrySummary[];
  tagPathOf: (tagIds: string[]) => string;
  commands: Command[];
  guides: Guide[];
  inGuide: boolean;
  mac?: boolean;
}

/**
 * Groups in a fixed order: inside a guide Entries, Commands, Guides, then the create row; outside
 * one Guides come first and there is no create row.
 */
export function buildGroups(input: string, sources: Sources): Group[] {
  const { query, commandsOnly } = parseInput(input);
  const q = query.toLowerCase();
  const groups: Group[] = [];

  const commands: Row[] = sources.commands
    .filter((c) => !q || c.label.toLowerCase().includes(q))
    .slice(0, 8)
    .map((command) => {
      const keys = command.keys !== undefined ? command.keys : shortcutFor(command.action);
      return { kind: "command", command, keys: keys ? displayKeys(keys, sources.mac) : null };
    });
  const guides: Row[] = sources.guides
    .filter((g) => !q || g.name.toLowerCase().includes(q))
    .slice(0, 6)
    .map((guide) => ({ kind: "guide", guide }));
  const entries: Row[] = sources.entries
    .slice(0, 8)
    .map((entry) => ({ kind: "entry", entry, detail: sources.tagPathOf(entry.tag_ids) }));

  if (commandsOnly) return commands.length > 0 ? [{ heading: "Commands", rows: commands }] : [];

  if (sources.inGuide) {
    if (entries.length > 0) groups.push({ heading: "Entries", rows: entries });
    if (commands.length > 0) groups.push({ heading: "Commands", rows: commands });
    if (guides.length > 0) groups.push({ heading: "Guides", rows: guides });
    if (q && !sources.entries.some((e) => e.title.toLowerCase() === q)) {
      groups.push({ heading: "New", rows: [{ kind: "create", title: query }] });
    }
  } else {
    if (guides.length > 0) groups.push({ heading: "Guides", rows: guides });
    if (commands.length > 0) groups.push({ heading: "Commands", rows: commands });
  }
  return groups;
}

export function flatten(groups: Group[]): Row[] {
  return groups.flatMap((g) => g.rows);
}

/** Views inside a guide, where guide commands apply. */
const GUIDE_VIEWS = ["guide", "entry", "tags", "trash", "boards", "board"];

/**
 * The commands the palette offers on a view. Page commands come first; on a board, Mod+F goes to
 * the board's picker search, so the key cap moves from "Search this guide" to "Add entries".
 */
export function commandsFor(view: string, inGuide: boolean, hasNotice: boolean): Command[] {
  const guideLevel = inGuide && GUIDE_VIEWS.includes(view);
  const list: Command[] = [];
  if (guideLevel) list.push({ action: "new", label: "New entry" });
  list.push({ action: "new-guide", label: "New guide" });
  if (view === "entry") {
    list.push(
      { action: "toggle-mode", label: "Toggle Read and Edit" },
      { action: "focus-relation", label: "Add a relation" },
      { action: "add-tag", label: "Add a tag" },
      { action: "rename", label: "Rename entry" },
      { action: "toggle-panel", label: "Show or hide the right panel" },
    );
  }
  if (view === "board") {
    list.push(
      { action: "board-add-entries", label: "Add entries", keys: shortcutFor("focus-search") },
      { action: "board-new-note", label: "New note" },
      { action: "board-fit", label: "Fit board" },
      { action: "toggle-panel", label: "Show or hide the right panel" },
    );
  }
  if (guideLevel) {
    list.push(
      view === "board"
        ? { action: "focus-search", label: "Search this guide", keys: null }
        : { action: "focus-search", label: "Search this guide" },
      { action: "tag-editor", label: "Tag editor" },
      { action: "boards", label: "Boards" },
      { action: "trash", label: "Trash" },
    );
  }
  list.push(
    { action: "toggle-sidebar", label: "Show or hide the sidebar" },
    { action: "settings", label: "Settings" },
    { action: "back", label: "Back" },
    { action: "forward", label: "Forward" },
  );
  if (hasNotice) list.push({ action: "dismiss-notice", label: "Dismiss notice" });
  list.push(
    { action: "density-compact", label: "Density: Compact" },
    { action: "density-roomy", label: "Density: Roomy" },
    { action: "theme-system", label: "Theme: System" },
    { action: "theme-light", label: "Theme: Light" },
    { action: "theme-dark", label: "Theme: Dark" },
  );
  return list;
}
