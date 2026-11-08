// Process-wide UI state: what the shell reported at startup, saved preferences, notices waiting
// to be shown, and the renames that can still be undone this session (design section 3, "Rename
// is undoable ... for the rest of the session").

import type { RenameRecord, Startup, UiPrefs } from "./api";

export interface Notice {
  id: number;
  text: string;
}

export const session = $state({
  startup: null as Startup | null,
  prefs: { guide_view: "cards" } as UiPrefs,
  notices: [] as Notice[],
  /** Most recent rename per entry id. */
  renames: {} as Record<string, RenameRecord>,
});

let nextNoticeId = 1;

/** Queues a message for the banner at the top of the window. */
export function notify(text: string): void {
  session.notices.push({ id: nextNoticeId++, text });
}

export function dismiss(id: number): void {
  session.notices = session.notices.filter((n) => n.id !== id);
}
