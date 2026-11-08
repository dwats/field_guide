// Process-wide UI state: what the shell reported at startup, saved preferences, notices waiting
// to be shown, and the renames that can still be undone this session (design section 3, "Rename
// is undoable ... for the rest of the session").

import type { RenameRecord, Startup, UiPrefs } from "./api";

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

export const session = $state({
  startup: null as Startup | null,
  prefs: { guide_view: "cards" } as UiPrefs,
  /** Queue for the one notice slot under the top bar; the first entry is the one shown. */
  notices: [] as Notice[],
  /** Most recent rename per entry id. */
  renames: {} as Record<string, RenameRecord>,
});

let nextNoticeId = 1;

/**
 * Queues a message for the notice slot. Info notices replace any earlier info notice, since
 * each one reports the latest thing that happened; warnings and errors stay until dismissed.
 */
export function notify(text: string, options: { kind?: NoticeKind; action?: NoticeAction } = {}): number {
  const kind = options.kind ?? "info";
  if (kind === "info") {
    session.notices = session.notices.filter((n) => n.kind !== "info");
  }
  const id = nextNoticeId++;
  session.notices.push({ id, kind, text, action: options.action });
  return id;
}

export function dismiss(id: number): void {
  session.notices = session.notices.filter((n) => n.id !== id);
}
