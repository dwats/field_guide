// Process-wide UI state: what the shell reported at startup, saved preferences, notices waiting
// to be shown, the renames that can still be undone this session (design section 3, "Rename is
// undoable ... for the rest of the session"), what the current view shows, and the sidebar's
// scope and list, which the guide page shares.

import { api, type Entry, type EntrySummary, type Guide, type RenameRecord, type Startup, type UiPrefs } from "./api";
import type { Scope } from "./sidebar";

export type NoticeKind = "info" | "warning" | "error";

export interface NoticeAction {
  label: string;
  run: () => void | Promise<void>;
}

export interface Notice {
  id: number;
  kind: NoticeKind;
  text: string;
  action?: NoticeAction;
}

export interface SidebarState {
  /** The guide the state belongs to; a different guide starts fresh. */
  guideId: string | null;
  scope: Scope;
  /** The search line: free text plus `#Tag/Path` chips (see lib/filter.ts). */
  search: string;
  stubsOnly: boolean;
  /** Expanded tag ids, kept for the session. */
  expanded: string[];
  /** What the scope and search produce; the guide page shows this same list. */
  entries: EntrySummary[];
  /** The entry last opened from the list, so opening it does not move the scope. */
  pickedId: string | null;
}

/** Every preference with its default, so a shell answer missing a field still has one. */
export const DEFAULT_PREFS: UiPrefs = {
  guide_view: "cards",
  density: "roomy",
  sidebar_collapsed: false,
  panel_collapsed: false,
  panel_open_in_edit: false,
  hide_contents: false,
  sidebar_width: null,
  panel_width: null,
  sidebar_split: null,
};

export function freshSidebar(guideId: string | null): SidebarState {
  return {
    guideId,
    scope: { kind: "all" },
    search: "",
    stubsOnly: false,
    expanded: [],
    entries: [],
    pickedId: null,
  };
}

export const session = $state({
  startup: null as Startup | null,
  prefs: { ...DEFAULT_PREFS } as UiPrefs,
  /** Queue for the one notice slot under the top bar; the first entry is the one shown. */
  notices: [] as Notice[],
  /** Most recent rename per entry id. */
  renames: {} as Record<string, RenameRecord>,
  /** The guide and entry the current view shows; the top bar and the sidebar read these. */
  current: { guide: null as Guide | null, entry: null as Entry | null },
  /** Bumped after a change the rest of the shell should reload for (tree, counts, lists). */
  revision: 0,
  sidebar: freshSidebar(null),
  /** The guides list's filter, set from its sidebar. */
  guidesFilter: { name: "", tagIds: [] as string[] },
});

let nextNoticeId = 1;

/**
 * Queues a message for the notice slot. An info notice reports the latest thing that happened,
 * so it goes to the front and replaces any earlier info notice; warnings and errors queue behind
 * and stay until dismissed.
 */
export function notify(text: string, options: { kind?: NoticeKind; action?: NoticeAction } = {}): number {
  const kind = options.kind ?? "info";
  const id = nextNoticeId++;
  const notice: Notice = { id, kind, text, action: options.action };
  if (kind === "info") {
    session.notices = [notice, ...session.notices.filter((n) => n.kind !== "info")];
  } else {
    session.notices.push(notice);
  }
  return id;
}

export function dismiss(id: number): void {
  session.notices = session.notices.filter((n) => n.id !== id);
}

/** Views call this once they know what they show. */
export function setCurrent(guide: Guide | null, entry: Entry | null = null): void {
  session.current = { guide, entry };
}

/** Marks that data changed, so the sidebar's tree, counts, and list reload. */
export function touch(): void {
  session.revision += 1;
}

/** Saves preferences through the shell and keeps the session copy in step. */
export async function savePrefs(update: Partial<UiPrefs>): Promise<void> {
  session.prefs = { ...DEFAULT_PREFS, ...(await api.setPrefs({ ...session.prefs, ...update })) };
}
